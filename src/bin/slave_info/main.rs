extern crate soem;
extern crate clap;

use clap::{App, Arg};
use soem::*;
use std::default::Default;
use std::os::raw::c_int;
use std::mem::zeroed;

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

	let matches = App::new("EtherCat slave info")
		.version("1.0")
		.author("Matwey V. Kornilov <matwey.kornilov@gmail.com>")
		.arg(Arg::with_name("iface").required(true))
		.get_matches();

	let mut io_map: [u8; 4096] = unsafe { zeroed() };

	match Context::new(matches.value_of("iface").unwrap(),
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
		&mut eep_fmmu) {

		Ok(ref mut c) => {
			match c.config_init(false) {
				Ok(_) => {
					c.config_map_group(&mut io_map, 0);
					c.config_dc();
					println!("{} slaves found and configured.", c.slave_count());
					let new_state = c.statecheck(0, EtherCatState::SafeOp, 20000 * 3);
					println!("new_state = {:?}", new_state);
					let lowest_state = c.readstate();
					println!("lowest_state = {:?}", lowest_state);
				},
				Err(ref err) => println!("Cannot configure EtherCat: {}", err),
			}
		},
		Err(ref err) => println!("Cannot create EtherCat context: {}", err),
	};
}
