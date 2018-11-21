extern crate soem;
use std::default::Default;
use soem::*;
use std::os::raw::c_int;

fn main() {
	let mut port: Port = Default::default();
	let mut slaves: [Slave ; 8] = Default::default();
	let mut slavecount: c_int = Default::default();
	let mut groups: [Group ; 2] = Default::default();
	let mut esibuf: ESIBuf = Default::default();
	let mut esimap: ESIMap = Default::default();
	let mut elist: ERing = Default::default();
	let mut idxstack: IdxStack = Default::default();
	let mut ecaterror: Boolean = Default::default();
	let mut dc_time: Int64 = Default::default();
	let mut sm_commtype: SMCommType = Default::default();
	let mut pdo_assign: PDOAssign = Default::default();
	let mut pdo_desc: PDODesc = Default::default();
	let mut eep_sm: EEPROMSM = Default::default();
	let mut eep_fmmu: EEPROMFMMU = Default::default();

	let c = Context::new("eth2",
		&mut port,
		&mut slaves,
		&mut slavecount,
		&mut groups,
		&mut esibuf,
		&mut esimap,
		&mut elist,
		&mut idxstack,
		&mut ecaterror,
		&mut dc_time,
		&mut sm_commtype,
		&mut pdo_assign,
		&mut pdo_desc,
		&mut eep_sm,
		&mut eep_fmmu).unwrap();

}
