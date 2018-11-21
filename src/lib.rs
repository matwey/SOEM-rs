extern crate SOEM_sys;

use std::marker::PhantomData;
use std::default::Default;
use std::mem::zeroed;
use std::os::raw::c_int;
use std::ffi::{CString, NulError};
use std::fmt;
use std::result;

use SOEM_sys::{
	boolean,
	ec_PDOassign,
	ec_PDOdesc,
	ec_SMcommtype,
	ec_eepromFMMU,
	ec_eepromSM,
	ec_ering,
	ec_group,
	ec_idxstack,
	ec_slave,
	ecx_close,
	ecx_context,
	ecx_init,
	ecx_portt,
	int16,
	int32,
	int64,
	uint16,
	uint32,
	uint64,
	uint8,
};

pub mod context;

/** size of EEPROM bitmap cache */
const EC_MAXEEPBITMAP : usize = 128;
/** size of EEPROM cache buffer */
const EC_MAXEEPBUF : usize = EC_MAXEEPBITMAP << 5;

pub type Boolean = boolean;
pub type Int16 = int16;
pub type Int32 = int32;
pub type Int64 = int64;
pub type UInt16 = uint16;
pub type UInt32 = uint32;
pub type UInt64 = uint64;
pub type UInt8 = uint8;

#[derive(Debug)]
pub enum Error {
	InitError,
	CStringError(NulError),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::InitError => write!(f, "Initialization error"),
			Error::CStringError(ref err) => write!(f, "CString error: {}", err),
		}
	}
}

type Result<T> = result::Result<T, Error>;

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

#[repr(C)]
pub struct Group(ec_group);

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
pub struct ERing(ec_ering);

impl Default for ERing {
	fn default() -> ERing {
		ERing(unsafe { zeroed() })
	}
}

#[repr(C)]
pub struct IdxStack(ec_idxstack);

impl Default for IdxStack {
	fn default() -> IdxStack {
		IdxStack(unsafe { zeroed() })
	}
}

#[repr(C)]
pub struct PDOAssign(ec_PDOassign);

impl Default for PDOAssign {
	fn default() -> PDOAssign {
		PDOAssign(unsafe { zeroed() })
	}
}

#[repr(C)]
pub struct PDODesc(ec_PDOdesc);

impl Default for PDODesc {
	fn default() -> PDODesc {
		PDODesc(unsafe { zeroed() })
	}
}

#[repr(C)]
pub struct SMCommType(ec_SMcommtype);

impl Default for SMCommType {
	fn default() -> SMCommType {
		SMCommType(unsafe { zeroed() })
	}
}

#[repr(C)]
pub struct EEPROMFMMU(ec_eepromFMMU);

impl Default for EEPROMFMMU {
	fn default() -> EEPROMFMMU {
		EEPROMFMMU(unsafe { zeroed() })
	}
}

#[repr(C)]
pub struct EEPROMSM(ec_eepromSM);

impl Default for EEPROMSM {
	fn default() -> EEPROMSM {
		EEPROMSM(unsafe { zeroed() })
	}
}

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
		ecaterror:  &'a mut Boolean,
		dc_time: &'a mut Int64,
		sm_commtype: &'a mut SMCommType,
		pdo_assign: &'a mut PDOAssign,
		pdo_desc: &'a mut PDODesc,
		eep_sm: &'a mut EEPROMSM,
		eep_fmmu: &'a mut EEPROMFMMU
	) -> Result<Self> {
		let mut c = Context {
			context: ecx_context {
				port:       &mut port.0,
				slavelist:  &mut slaves[0].0,
				slavecount: &mut *slavecount,
				maxslave:   slaves.len() as c_int,
				grouplist:  &mut groups[0].0,
				maxgroup:   groups.len() as c_int,
				esibuf:     esibuf.0.as_mut_ptr(),
				esimap:     esimap.0.as_mut_ptr(),
				esislave:   Default::default(),
				elist:      &mut elist.0,
				idxstack:   &mut idxstack.0,
				ecaterror:  &mut *ecaterror,
				DCtO:       Default::default(),
				DCl:        Default::default(),
				DCtime:     &mut *dc_time,
				SMcommtype: &mut sm_commtype.0,
				PDOassign:  &mut pdo_assign.0,
				PDOdesc:    &mut pdo_desc.0,
				eepSM:      &mut eep_sm.0,
				eepFMMU:    &mut eep_fmmu.0,
				FOEhook:    Default::default()
			},
			_phantom: Default::default()
		};

		CString::new(iface_name)
			.map_err(|err| Error::CStringError(err))
			.and_then(|iface| {
				match unsafe { ecx_init(&mut c.context, iface.as_ptr()) } {
					1 => Ok(c),
					_ => Err(Error::InitError),
				}
			})
	}
}
