mod error;

use boolinator::Boolinator;

#[macro_use]
extern crate num_derive;

use std::{
    borrow::Cow,
    default::Default,
    ffi::{CStr, CString},
    fmt, mem,
    mem::zeroed,
    ops::Not,
    os::raw::c_int,
    result, slice,
};

use crate::error::{ErrorGenerator, ErrorIterator, EtherCatError, InitError};

#[rustfmt::skip]
use SOEM_sys::{
    self as soem,
	boolean,
	ec_PDOassignt,
	ec_PDOdesct,
	ec_SMcommtypet,
	ec_eepromFMMUt,
	ec_eepromSMt,
	ec_eringt,
	ec_group,
	ec_idxstackT,
	ec_slave,
	ec_state_EC_STATE_BOOT,
	ec_state_EC_STATE_ERROR,
	ec_state_EC_STATE_INIT,
	ec_state_EC_STATE_NONE,
	ec_state_EC_STATE_OPERATIONAL,
	ec_state_EC_STATE_PRE_OP,
	ec_state_EC_STATE_SAFE_OP,
	ecx_SDOread,
	ecx_SDOwrite,
	ecx_close,
	ecx_config_init,
	ecx_config_map_group,
	ecx_configdc,
	ecx_context,
	ecx_elist2string,
	ecx_init,
	ecx_iserror,
	ecx_portt,
	ecx_receive_processdata,
	ecx_send_processdata,
	ecx_statecheck,
	ecx_readstate,
	ecx_writestate,
};

#[derive(FromPrimitive, Debug, PartialEq, Clone, Copy)]
#[repr(u16)]
pub enum EtherCatState {
    /// Boot state
    Boot = ec_state_EC_STATE_BOOT as u16,
    /// Init state
    Init = ec_state_EC_STATE_INIT as u16,
    /// No valid state
    None = ec_state_EC_STATE_NONE as u16,
    /// Error or ACK error
    AckOrError = ec_state_EC_STATE_ERROR as u16,
    /// Operational
    Op = ec_state_EC_STATE_OPERATIONAL as u16,
    /// Pre-operational
    PreOp = ec_state_EC_STATE_PRE_OP as u16,
    /// Safe-operational
    SafeOp = ec_state_EC_STATE_SAFE_OP as u16,
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
#[derive(Clone)]
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
    pub const fn output_size(&self) -> u16 {
        self.0.Obits
    }
    pub const fn input_size(&self) -> u16 {
        self.0.Ibits
    }
    pub fn outputs(&self) -> &mut [u8] {
        let size = (if self.0.Obytes == 0 && self.0.Obits > 0 {
            1
        } else {
            self.0.Obytes
        }) as usize;
        unsafe { slice::from_raw_parts_mut(self.0.outputs, size) }
    }
    pub fn inputs(&self) -> &[u8] {
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
    pub const fn prop_delay(&self) -> i32 {
        self.0.pdelay
    }
    pub const fn has_dc(&self) -> bool {
        self.0.hasdc != 0
    }
    pub const fn eep_manufacturer(&self) -> u32 {
        self.0.eep_man
    }
    pub const fn eep_id(&self) -> u32 {
        self.0.eep_id
    }
    pub const fn eep_revision(&self) -> u32 {
        self.0.eep_rev
    }
    pub const fn parent_port(&self) -> u8 {
        self.0.parentport
    }
    pub const fn configured_addr(&self) -> u16 {
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
#[derive(Clone)]
pub struct Group(ec_group);

impl Group {
    pub const fn outputs_wkc(&self) -> u16 {
        self.0.outputsWKC
    }
    pub const fn inputs_wkc(&self) -> u16 {
        self.0.inputsWKC
    }
    pub const fn expected_wkc(&self) -> u16 {
        self.outputs_wkc() * 2 + self.inputs_wkc()
    }
}

impl Default for Group {
    fn default() -> Group {
        Group(unsafe { zeroed() })
    }
}

/// Size of EEPROM bitmap cache
const EC_MAXEEPBITMAP: usize = 128;

/// Size of EEPROM cache buffer
const EC_MAXEEPBUF: usize = EC_MAXEEPBITMAP << 5;

#[repr(C)]
pub struct ESIBuf([u8; EC_MAXEEPBUF]);

impl Default for ESIBuf {
    fn default() -> ESIBuf {
        Self([0; EC_MAXEEPBUF])
    }
}

#[repr(C)]
pub struct ESIMap([u32; EC_MAXEEPBITMAP]);

impl Default for ESIMap {
    fn default() -> ESIMap {
        Self([0; EC_MAXEEPBITMAP])
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

pub struct Context {
    _port: Port,
    _esibuf: ESIBuf,
    _esimap: ESIMap,
    _elist: ERing,
    _ecaterror: u8,
    _dc_time: i64,
    _slavecount: i32,
    _idxstack: IdxStack,
    _sm_commtype: SMCommType,
    _pdo_assign: PDOAssign,
    _pdo_desc: PDODesc,
    _eep_sm: EEPROMSM,
    _eep_fmmu: EEPROMFMMU,
    _slaves: Vec<Slave>,
    _groups: Vec<Group>,
    ctx: ecx_context,
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ctx.fmt(f)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { ecx_close(&mut self.ctx) };
    }
}

impl Default for Context {
    fn default() -> Self {
        let mut _port: Port = Default::default();
        let mut _slaves = vec![Slave::default(); 8];
        let mut _slavecount: c_int = Default::default();
        let mut _groups = vec![Group::default(); 2];
        let mut _esibuf: ESIBuf = Default::default();
        let mut _esimap: ESIMap = Default::default();
        let mut _elist: ERing = Default::default();
        let mut _idxstack: IdxStack = Default::default();
        let mut _ecaterror = 0;
        let mut _dc_time = 0;
        let mut _sm_commtype: SMCommType = Default::default();
        let mut _pdo_assign: PDOAssign = Default::default();
        let mut _pdo_desc: PDODesc = Default::default();
        let mut _eep_sm: EEPROMSM = Default::default();
        let mut _eep_fmmu: EEPROMFMMU = Default::default();

        let ctx = ecx_context {
            port: &mut _port.0,
            slavelist: &mut _slaves[0].0,
            slavecount: &mut _slavecount,
            maxslave: _slaves.len() as i32,
            grouplist: &mut _groups[0].0,
            maxgroup: _groups.len() as i32,
            esibuf: _esibuf.0.as_mut_ptr(),
            esimap: _esimap.0.as_mut_ptr(),
            esislave: 0,
            elist: &mut _elist.0,
            idxstack: &mut _idxstack.0,
            ecaterror: &mut _ecaterror,
            DCtO: Default::default(),
            DCl: Default::default(),
            DCtime: &mut _dc_time,
            SMcommtype: &mut _sm_commtype.0,
            PDOassign: &mut _pdo_assign.0,
            PDOdesc: &mut _pdo_desc.0,
            eepSM: &mut _eep_sm.0,
            eepFMMU: &mut _eep_fmmu.0,
            FOEhook: Default::default(),
            EOEhook: Default::default(),
            manualstatechange: Default::default(),
        };

        Self {
            _port,
            _esibuf,
            _esimap,
            _elist,
            _ecaterror,
            _dc_time,
            _slavecount,
            _idxstack,
            _sm_commtype,
            _pdo_assign,
            _pdo_desc,
            _eep_sm,
            _eep_fmmu,
            _slaves,
            _groups,
            ctx,
        }
    }
}

impl Context {
    pub fn init(&mut self, iface: &str) -> Result<(), InitError> {
        CString::new(iface)
            .map_err(InitError::CStringError)
            .and_then(
                |iface| match unsafe { ecx_init(&mut self.ctx, iface.as_ptr()) } {
                    x if x > 0 => Ok(()),
                    _ => Err(InitError::IOError(std::io::Error::last_os_error())),
                },
            )
    }

    pub fn config_init(&mut self, usetable: bool) -> result::Result<usize, EtherCatError> {
        match unsafe { ecx_config_init(&mut self.ctx, usetable as soem::uint8) } {
            x if x > 0 => Ok(x as usize),
            x => Err(EtherCatError::from_code(x).unwrap()),
        }
    }

    pub fn config_map_group<'b>(
        &'b mut self,
        io_map: &mut [u8; 4096],
        group: u8,
    ) -> result::Result<usize, ErrorIterator<'b>> {
        let iomap_size = unsafe {
            ecx_config_map_group(
                &mut self.ctx,
                io_map.as_mut_ptr() as *mut std::ffi::c_void,
                group as soem::uint8,
            ) as usize
        };
        self.iserror()
            .not()
            .as_result(iomap_size, ErrorIterator::new(self))
    }

    pub fn config_dc<'b>(&'b mut self) -> Result<bool, ErrorIterator<'b>> {
        let has_dc = unsafe { ecx_configdc(&mut self.ctx) != 0 };
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
        let new_state = unsafe { ecx_statecheck(&mut self.ctx, slave, state as u16, timeout) };
        num::FromPrimitive::from_u16(new_state).unwrap()
    }

    pub fn read_state(&mut self) -> EtherCatState {
        let lowest_state = unsafe { ecx_readstate(&mut self.ctx) as u16 };
        num::FromPrimitive::from_u16(lowest_state).unwrap()
    }

    pub fn write_state(&mut self, slave: u16) -> Result<u16, EtherCatError> {
        let ret = unsafe { ecx_writestate(&mut self.ctx, slave) };
        match EtherCatError::from_code(ret) {
            Ok(err) => Err(err),
            Err(wck) => Ok(wck as u16),
        }
    }

    pub fn set_state(&mut self, state: EtherCatState, slave: u16) {
        self._slaves[slave as usize].0.state = state as u16;
    }

    pub fn dc_time(&mut self) -> i64 {
        unsafe { *self.ctx.DCtime }
    }

    pub fn slaves<'a>(&mut self) -> &'a [Slave] {
        unsafe {
            slice::from_raw_parts(
                (self.ctx.slavelist as *const Slave).offset(1),
                *self.ctx.slavecount as usize,
            )
        }
    }

    pub fn groups(&mut self) -> &[Group] {
        unsafe {
            slice::from_raw_parts(
                self.ctx.grouplist as *const Group,
                self.ctx.maxgroup as usize,
            )
        }
    }

    pub fn send_processdata(&mut self) {
        unsafe { ecx_send_processdata(&mut self.ctx) };
    }

    pub fn receive_processdata(&mut self, timeout: c_int) -> u16 {
        unsafe { ecx_receive_processdata(&mut self.ctx, timeout) as u16 }
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
                &mut self.ctx,
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
                &mut self.ctx,
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

impl ErrorGenerator for Context {
    fn iserror(&mut self) -> bool {
        unsafe { ecx_iserror(&mut self.ctx) != 0 }
    }
    fn next(&mut self) -> Option<String> {
        self.iserror().as_some(unsafe {
            CStr::from_ptr(ecx_elist2string(&mut self.ctx))
                .to_string_lossy()
                .into_owned()
        })
    }
}
