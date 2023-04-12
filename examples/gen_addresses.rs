use bech32::{self, ToBase32};
use clap::{App, Arg};

use std::error::Error;

use iota_ledger_nano::LedgerBIP32Index;

const HARDENED: u32 = 0x80000000;

const BIP32_CHANGE: u32 = /*0 |*/ HARDENED;
const BIP32_INDEX: u32 = /*0 |*/ HARDENED;

pub fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new("ledger address generator")
        .version("1.0")
        .author("Alexander Sporn <alexander.sporn@iota.org>")
        .arg(
            Arg::with_name("coin-type")
                .short("c")
                .long("coin-type")
                .help("select coin type (iota, smr) (default smr)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("number")
                .short("n")
                .help("number of addresses (default 5)")
                .takes_value(true),
        )
        .get_matches();

    let transport_type = iota_ledger_nano::TransportTypes::NativeHID;

    let hrp;
    let chain;

    (hrp, chain) = match matches.value_of("coin-type") {
        Some(c) => match c {
            "iota" => ("iota", 0x107a),
            "smr" => ("smr", 0x107b),
            "rms" => ("rms", 0x1),
            "atoi" => ("atoi", 0x1),
            _ => panic!("unknown coin type"),
        },
        None => ("smr", 0x107b),
    };

    let count = match matches.value_of("number") {
        Some(c) => c.parse::<u32>().unwrap(),
        None => 5,
    };

    for n in 0..count {
        let account = n | 0x80000000;

        let ledger = iota_ledger_nano::get_ledger_by_type(chain, account, &transport_type, None)?;

        let bip32_indices = LedgerBIP32Index {
            bip32_change: BIP32_CHANGE,
            bip32_index: BIP32_INDEX,
        };

        // generate address without prompt
        let addresses = ledger.get_addresses(false, bip32_indices, 1)?;
        let address_bytes = match addresses.first() {
            Some(a) => a,
            None => panic!("no address was generated!"),
        };

        let mut addr_bytes_with_type = [0u8; 33];

        // first address byte is 0 for ed25519
        addr_bytes_with_type[1..33].clone_from_slice(&address_bytes[..]);

        let bech32_address = bech32::encode(hrp, addr_bytes_with_type.to_base32()).unwrap();

        println!(
            "wallet address (2c'/{:x}'/{:x}'/{:x}'/{:x}'): {}",
            chain,
            account & !HARDENED,
            BIP32_CHANGE & !HARDENED,
            BIP32_INDEX & !HARDENED,
            bech32_address
        );
    }

    Ok(())
}
