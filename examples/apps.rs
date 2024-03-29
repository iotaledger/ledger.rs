use clap::{App, Arg};
use std::{thread, time};

use std::error::Error;

pub fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new("ledger iota tester")
        .version("1.0")
        .author("Thomas Pototschnig <microengineer18@gmail.com>")
        .arg(
            Arg::with_name("is-simulator")
                .short("s")
                .long("simulator")
                .value_name("is_simulator")
                .help("select the simulator as transport")
                .takes_value(false),
        )
        .get_matches();

    let is_simulator = matches.is_present("is-simulator");

    let transport_type = if is_simulator {
        iota_ledger_nano::TransportTypes::TCP
    } else {
        iota_ledger_nano::TransportTypes::NativeHID
    };

    let (app, version) = iota_ledger_nano::get_opened_app(&transport_type)?;
    println!("current app: {} {}", app, version);

    if app != "BOLOS" {
        iota_ledger_nano::exit_app(&transport_type)?;
        thread::sleep(time::Duration::from_secs(5));
        println!("current app: {} {}", app, version);
    }

    if app != "IOTA" {
        iota_ledger_nano::open_app(&transport_type, String::from("IOTA"))?;
        thread::sleep(time::Duration::from_secs(5));
    }

    let (app, version) = iota_ledger_nano::get_opened_app(&transport_type)?;
    println!("current app: {} {}", app, version);
    Ok(())
}
