extern crate SOEM_sys;
extern crate boolinator;

mod error;

use boolinator::Boolinator;

extern crate num;
#[macro_use]
extern crate num_derive;
use std::borrow::Cow;
use std::default::Default;
use std::ffi::{CStr, CString};
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::mem::zeroed;
use std::ops::Not;
use std::os::raw::c_int;
use std::result;
use std::slice;

use crate::error::{ErrorGenerator, ErrorIterator, EtherCatError, InitError};

use SOEM_sys::{
    boolean, ec_PDOassignt, ec_PDOdesct, ec_SMcommtypet, ec_eepromFMMUt, ec_eepromSMt, ec_eringt,
    ec_group, ec_idxstackT, ec_slave, ec_state_EC_STATE_BOOT, ec_state_EC_STATE_ERROR,
    ec_state_EC_STATE_INIT, ec_state_EC_STATE_NONE, ec_state_EC_STATE_OPERATIONAL,
    ec_state_EC_STATE_PRE_OP, ec_state_EC_STATE_SAFE_OP, ecx_SDOread, ecx_SDOwrite, ecx_close,
    ecx_config_init, ecx_config_map_group, ecx_configdc, ecx_context, ecx_elist2string, ecx_init,
    ecx_iserror, ecx_portt, ecx_readstate, ecx_receive_processdata, ecx_send_processdata,
    ecx_statecheck, ecx_writestate, int16, int32, int64, uint16, uint32, uint64, uint8,
};

/** size of EEPROM bitmap cache */
const EC_MAXEEPBITMAP: usize = 128;
/** size of EEPROM cache buffer */
const EC_MAXEEPBUF: usize = EC_MAXEEPBITMAP << 5;

pub type Boolean = boolean;
pub type Int16 = int16;
pub type Int32 = int32;
pub type Int64 = int64;
pub type UInt16 = uint16;
pub type UInt32 = uint32;
pub type UInt64 = uint64;
pub type UInt8 = uint8;

#[derive(FromPrimitive, Debug)]
#[repr(u16)]
pub enum EtherCatState {
    Boot = ec_state_EC_STATE_BOOT as u16,        // Boot state
    Init = ec_state_EC_STATE_INIT as u16,        // Init state
    None = ec_state_EC_STATE_NONE as u16,        // No valid state
    AckOrError = ec_state_EC_STATE_ERROR as u16, // Error or ACK error
    Op = ec_state_EC_STATE_OPERATIONAL as u16,   // Operational
    PreOp = ec_state_EC_STATE_PRE_OP as u16,     // Pre-operational
    SafeOp = ec_state_EC_STATE_SAFE_OP as u16,   // Safe-operational
}

impl fmt::Display for EtherCatState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EtherCatState::Boot => write!(f, "Boot"),
            EtherCatState::Init => write!(f, "Init"),
            EtherCatState::None => write!(f, "None"),
            EtherCatState::AckOrError => write!(f, "Ack or Error"),
            EtherCatState::Op => write!(f, "Operational"),
            EtherCatState::PreOp => write!(f, "Pre-Operational"),
            EtherCatState::SafeOp => write!(f, "Safe-Operational"),
        }
    }
}

#[repr(C)]
pub struct Port(ecx_portt);

impl Default for Port {
    fn default() -> Port {
        Port(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct Slave(ec_slave);

impl Default for Slave {
    fn default() -> Slave {
        Slave(unsafe { zeroed() })
    }
}

impl Slave {
    pub fn name(&self) -> Cow<str> {
        let name_str = unsafe { CStr::from_ptr(self.0.name.as_ptr()) };
        name_str.to_string_lossy()
    }
    pub fn output_size(&self) -> u16 {
        self.0.Obits
    }
    pub fn input_size(&self) -> u16 {
        self.0.Ibits
    }
    pub fn outputs<'a>(&'a self) -> &'a mut [u8] {
        let size = (if self.0.Obytes == 0 && self.0.Obits > 0 {
            1
        } else {
            self.0.Obytes
        }) as usize;
        unsafe { slice::from_raw_parts_mut(self.0.outputs, size) }
    }
    pub fn inputs<'a>(&'a self) -> &'a [u8] {
        let size = (if self.0.Ibytes == 0 && self.0.Ibits > 0 {
            1
        } else {
            self.0.Ibytes
        }) as usize;
        unsafe { slice::from_raw_parts_mut(self.0.inputs, size) }
    }
    pub fn state(&self) -> EtherCatState {
        num::FromPrimitive::from_u16(self.0.state).unwrap()
    }
    pub fn prop_delay(&self) -> i32 {
        self.0.pdelay
    }
    pub fn has_dc(&self) -> bool {
        self.0.hasdc != 0
    }
    pub fn eep_manufacturer(&self) -> u32 {
        self.0.eep_man
    }
    pub fn eep_id(&self) -> u32 {
        self.0.eep_id
    }
    pub fn eep_revision(&self) -> u32 {
        self.0.eep_rev
    }
    pub fn parent_port(&self) -> u8 {
        self.0.parentport
    }
    pub fn configured_addr(&self) -> u16 {
        self.0.configadr
    }
}

impl fmt::Display for Slave {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, " Name: {}\n Output size: {}bits\n Input size: {}bits\n State: {}\n Delay: {}[ns]\n Has DC: {}",
			self.name(),
			self.output_size(),
			self.input_size(),
			self.state(),
			self.prop_delay(),
			self.has_dc())?;
        if self.has_dc() {
            writeln!(f, " DCParentport: {}", self.parent_port())?;
        }
        writeln!(f, " Configured address: {:04x}", self.configured_addr())?;
        writeln!(
            f,
            " Man: {:08x} ID: {:08x} Rev: {:08x}",
            self.eep_manufacturer(),
            self.eep_id(),
            self.eep_revision()
        )
    }
}

#[repr(C)]
pub struct Group(ec_group);

impl Group {
    pub fn outputs_wkc(&self) -> u16 {
        self.0.outputsWKC
    }
    pub fn inputs_wkc(&self) -> u16 {
        self.0.inputsWKC
    }
    pub fn expected_wkc(&self) -> u16 {
        self.outputs_wkc() * 2 + self.inputs_wkc()
    }
}

impl Default for Group {
    fn default() -> Group {
        Group(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct ESIBuf([UInt8; EC_MAXEEPBUF]);

impl Default for ESIBuf {
    fn default() -> ESIBuf {
        ESIBuf(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct ESIMap([UInt32; EC_MAXEEPBITMAP]);

impl Default for ESIMap {
    fn default() -> ESIMap {
        ESIMap(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct ERing(ec_eringt);

impl Default for ERing {
    fn default() -> ERing {
        ERing(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct IdxStack(ec_idxstackT);

impl Default for IdxStack {
    fn default() -> IdxStack {
        IdxStack(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct PDOAssign(ec_PDOassignt);

impl Default for PDOAssign {
    fn default() -> PDOAssign {
        PDOAssign(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct PDODesc(ec_PDOdesct);

impl Default for PDODesc {
    fn default() -> PDODesc {
        PDODesc(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct SMCommType(ec_SMcommtypet);

impl Default for SMCommType {
    fn default() -> SMCommType {
        SMCommType(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct EEPROMFMMU(ec_eepromFMMUt);

impl Default for EEPROMFMMU {
    fn default() -> EEPROMFMMU {
        EEPROMFMMU(unsafe { zeroed() })
    }
}

#[repr(C)]
pub struct EEPROMSM(ec_eepromSMt);

impl Default for EEPROMSM {
    fn default() -> EEPROMSM {
        EEPROMSM(unsafe { zeroed() })
    }
}

#[derive(Debug)]
pub struct Context<'a> {
    context: ecx_context,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Drop for Context<'a> {
    fn drop(&mut self) {
        unsafe { ecx_close(&mut self.context) };
    }
}

impl<'a> Context<'a> {
    pub fn new(
        iface_name: &str,
        port: &'a mut Port,
        slaves: &'a mut [Slave],
        slavecount: &'a mut c_int,
        groups: &'a mut [Group],
        esibuf: &'a mut ESIBuf,
        esimap: &'a mut ESIMap,
        elist: &'a mut ERing,
        idxstack: &'a mut IdxStack,
        ecaterror: &'a mut Boolean,
        dc_time: &'a mut Int64,
        sm_commtype: &'a mut SMCommType,
        pdo_assign: &'a mut PDOAssign,
        pdo_desc: &'a mut PDODesc,
        eep_sm: &'a mut EEPROMSM,
        eep_fmmu: &'a mut EEPROMFMMU,
    ) -> result::Result<Self, InitError> {
        let mut c = Context {
            context: ecx_context {
                port: &mut port.0,
                slavelist: &mut slaves[0].0,
                slavecount: &mut *slavecount,
                maxslave: slaves.len() as c_int,
                grouplist: &mut groups[0].0,
                maxgroup: groups.len() as c_int,
                esibuf: esibuf.0.as_mut_ptr(),
                esimap: esimap.0.as_mut_ptr(),
                esislave: Default::default(),
                elist: &mut elist.0,
                idxstack: &mut idxstack.0,
                ecaterror: &mut *ecaterror,
                DCtO: Default::default(),
                DCl: Default::default(),
                DCtime: &mut *dc_time,
                SMcommtype: &mut sm_commtype.0,
                PDOassign: &mut pdo_assign.0,
                PDOdesc: &mut pdo_desc.0,
                eepSM: &mut eep_sm.0,
                eepFMMU: &mut eep_fmmu.0,
                FOEhook: Default::default(),
                EOEhook: Default::default(),
                manualstatechange: Default::default(),
            },
            _phantom: Default::default(),
        };

        CString::new(iface_name)
            .map_err(|err| InitError::CStringError(err))
            .and_then(
                |iface| match unsafe { ecx_init(&mut c.context, iface.as_ptr()) } {
                    x if x > 0 => Ok(c),
                    _ => Err(InitError::IOError(std::io::Error::last_os_error())),
                },
            )
    }

    pub fn config_init(&mut self, usetable: bool) -> result::Result<usize, EtherCatError> {
        match unsafe { ecx_config_init(&mut self.context, usetable as UInt8) } {
            x if x > 0 => Ok(x as usize),
            x => Err(EtherCatError::from_code(x).unwrap()),
        }
    }

    pub fn config_map_group<'b>(
        &'b mut self,
        io_map: &'a mut [u8; 4096],
        group: u8,
    ) -> result::Result<usize, ErrorIterator<'b>> {
        let iomap_size = unsafe {
            ecx_config_map_group(
                &mut self.context,
                io_map.as_mut_ptr() as *mut std::ffi::c_void,
                group as UInt8,
            ) as usize
        };
        self.iserror()
            .not()
            .as_result(iomap_size, ErrorIterator::new(self))
    }

    pub fn config_dc<'b>(&'b mut self) -> result::Result<bool, ErrorIterator<'b>> {
        let has_dc = unsafe { ecx_configdc(&mut self.context) != 0 };
        self.iserror()
            .not()
            .as_result(has_dc, ErrorIterator::new(self))
    }

    pub fn check_state(
        &mut self,
        slave: u16,
        state: EtherCatState,
        timeout: c_int,
    ) -> EtherCatState {
        let new_state = unsafe { ecx_statecheck(&mut self.context, slave, state as u16, timeout) };
        num::FromPrimitive::from_u16(new_state).unwrap()
    }

    pub fn read_state(&mut self) -> EtherCatState {
        let lowest_state = unsafe { ecx_readstate(&mut self.context) as u16 };
        num::FromPrimitive::from_u16(lowest_state).unwrap()
    }

    pub fn write_state(&mut self, slave: u16) -> result::Result<u16, EtherCatError> {
        let ret = unsafe { ecx_writestate(&mut self.context, slave) };
        match EtherCatError::from_code(ret) {
            Ok(err) => Err(err),
            Err(wck) => Ok(wck as u16),
        }
    }

    pub fn set_state(&mut self, state: EtherCatState, slave: u16) {
        let raw_slaves = unsafe {
            slice::from_raw_parts_mut(self.context.slavelist, *self.context.slavecount as usize)
        };
        raw_slaves[slave as usize].state = state as u16;
    }

    pub fn dc_time(&mut self) -> i64 {
        unsafe { *self.context.DCtime }
    }

    pub fn slaves(&mut self) -> &'a [Slave] {
        unsafe {
            slice::from_raw_parts(
                (self.context.slavelist as *const Slave).offset(1),
                *self.context.slavecount as usize,
            )
        }
    }

    pub fn groups(&mut self) -> &'a [Group] {
        unsafe {
            slice::from_raw_parts(
                self.context.grouplist as *const Group,
                self.context.maxgroup as usize,
            )
        }
    }

    pub fn send_processdata(&mut self) {
        unsafe { ecx_send_processdata(&mut self.context) };
    }

    pub fn receive_processdata(&mut self, timeout: c_int) -> u16 {
        unsafe { ecx_receive_processdata(&mut self.context, timeout) as u16 }
    }

    pub fn write_sdo<'b, T: num::PrimInt + ?Sized>(
        &'b mut self,
        slave: u16,
        index: u16,
        subindex: u8,
        value: &T,
        timeout: c_int,
    ) -> result::Result<(), ErrorIterator<'b>> {
        let mut value_le = value.to_le();
        let psize = mem::size_of_val(&value_le) as c_int;
        let value_ptr = &mut value_le as *mut T;

        unsafe {
            ecx_SDOwrite(
                &mut self.context,
                slave,
                index,
                subindex,
                0 as boolean,
                psize,
                value_ptr as *mut std::ffi::c_void,
                timeout,
            )
        };

        self.iserror().not().as_result((), ErrorIterator::new(self))
    }

    pub fn read_sdo<'b, T: num::PrimInt + ?Sized>(
        &'b mut self,
        slave: u16,
        index: u16,
        subindex: u8,
        timeout: c_int,
    ) -> result::Result<T, ErrorIterator<'b>> {
        let mut value_le: T = unsafe { zeroed() };
        let mut psize = mem::size_of_val(&value_le) as c_int;
        let psize_ptr = &mut psize as *mut c_int;
        let value_ptr = &mut value_le as *mut T;

        unsafe {
            ecx_SDOread(
                &mut self.context,
                slave,
                index,
                subindex,
                0 as boolean,
                psize_ptr,
                value_ptr as *mut std::ffi::c_void,
                timeout,
            )
        };

        self.iserror()
            .not()
            .as_result(num::PrimInt::from_le(value_le), ErrorIterator::new(self))
    }
}

impl<'a> ErrorGenerator for Context<'a> {
    fn iserror(&mut self) -> bool {
        unsafe { ecx_iserror(&mut self.context) != 0 }
    }
    fn next(&mut self) -> Option<String> {
        self.iserror().as_some(unsafe {
            CStr::from_ptr(ecx_elist2string(&mut self.context))
                .to_string_lossy()
                .into_owned()
        })
    }
}
