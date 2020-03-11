extern crate clap;
extern crate soem;

use clap::{App, Arg};
use soem::*;
use std::{iter::Iterator, mem::zeroed, thread::sleep, time::Duration};

fn simple_test(iface_name: &str) -> i32 {
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

    let expected_wkc = c.groups()[0].expected_wkc();
    println!("Calculated workcounter {}\n", expected_wkc);

    c.send_processdata();
    c.receive_processdata(2000);

    println!("Request {} state for the slaves", EtherCatState::Op);
    c.set_state(EtherCatState::Op, 0);
    match c.write_state(0) {
        Err(ref err) => {
            println!("Cannot set state for the slaves: {}", err);
            return 1;
        }
        Ok(_) => (),
    };

    let r#try = 40;
    for _ in 0..r#try {
        match c.check_state(0, EtherCatState::Op, 20000 * 3) {
            EtherCatState::Op => break,
            _ => {
                c.send_processdata();
                c.receive_processdata(2000);
            }
        }
    }

    match c.read_state() {
        EtherCatState::Op => (),
        _ => {
            println!("Cannot reach {} state for the slaves", EtherCatState::Op);
            for (i, s) in c.slaves().iter().enumerate() {
                match s.state() {
                    EtherCatState::Op => continue,
                    state => {
                        println!("Slave {} ({}) in state {}", i, s.name(), state);
                    }
                }
            }
            return 1;
        }
    }

    println!("Operational state reached for all slaves.");

    for i in 1..10000 {
        c.send_processdata();
        let wck = c.receive_processdata(2000);

        if wck >= expected_wkc {
            print!("Processdata cycle {}, ", i);
            for s in c.slaves().iter() {
                for x in s.inputs() {
                    print!("{:02x}", x);
                }
                print!(" ");
            }
            println!("WKC {} T:{}", wck, c.dc_time());
        }

        sleep(Duration::from_micros(5000));
    }

    println!("Request {} state for the slaves", EtherCatState::Init);
    c.set_state(EtherCatState::Init, 0);
    match c.write_state(0) {
        Err(ref err) => {
            println!("Cannot set state for the slaves: {}", err);
            return 1;
        }
        Ok(_) => (),
    };

    0
}

fn main() {
    let matches = App::new("EtherCat slave info")
        .version("1.0")
        .author("Matwey V. Kornilov <matwey.kornilov@gmail.com>")
        .arg(Arg::with_name("iface").required(true))
        .get_matches();

    let exit_code = simple_test(matches.value_of("iface").unwrap());
    std::process::exit(exit_code);
}
