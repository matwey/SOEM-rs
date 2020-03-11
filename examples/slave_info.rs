extern crate clap;
extern crate soem;

use clap::{App, Arg};
use soem::*;
use std::{default::Default, iter::Iterator, mem::zeroed};

fn slave_info(iface_name: &str) -> i32 {
    let mut io_map: [u8; 4096] = unsafe { zeroed() };

    let mut c = Context::default();

    match c.init(iface_name) {
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
