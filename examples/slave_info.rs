extern crate clap;
extern crate soem;

use clap::{App, Arg};
use soem::*;
use std::default::Default;
use std::iter::Iterator;
use std::mem::zeroed;
use std::os::raw::c_int;

fn slave_info(iface_name: &str) -> i32 {
    let mut port: Port = Default::default();
    let mut slaves: [Slave; 8] = Default::default();
    let mut slavecount: c_int = Default::default();
    let mut groups: [Group; 2] = Default::default();
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

    let mut io_map: [u8; 4096] = unsafe { zeroed() };

    let mut c = match Context::new(
        iface_name,
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
        &mut eep_fmmu,
    ) {
        Err(ref err) => {
            println!("Cannot create EtherCat context: {}", err);
            return 1;
        }
        Ok(c) => c,
    };

    match c.config_init(false) {
        Err(ref err) => {
            println!("Cannot configure EtherCat: {}", err);
            return 1;
        }
        Ok(_) => (),
    };

    match c.config_map_group(&mut io_map, 0) {
        Err(ref err) => {
            println!("Cannot configure group map: {}", err);
            return 1;
        }
        Ok(_) => (),
    };

    match c.config_dc() {
        Err(ref err) => {
            println!("Cannot configure DC: {}", err);
            return 1;
        }
        Ok(_) => (),
    };

    println!("{} slaves found and configured.", c.slaves().len());

    c.check_state(0, EtherCatState::SafeOp, 20000 * 3);
    let lowest_state = c.read_state();

    println!("Slaves state = {}", lowest_state);

    for (i, s) in c.slaves().iter().enumerate() {
        println!("Slave {}", i);
        println!("{}", s);
    }

    0
}

fn main() {
    let matches = App::new("EtherCat slave info")
        .version("1.0")
        .author("Matwey V. Kornilov <matwey.kornilov@gmail.com>")
        .arg(Arg::with_name("iface").required(true))
        .get_matches();

    let exit_code = slave_info(matches.value_of("iface").unwrap());
    std::process::exit(exit_code);
}
