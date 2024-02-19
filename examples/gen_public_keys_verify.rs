use bech32::{self, ToBase32};
use clap::{App, Arg};
use iota_ledger_nano::api::constants::{CoinType, Protocols};

use std::error::Error;

use blake2::digest::{Update, VariableOutput};
use blake2::VarBlake2b;
use iota_ledger_nano::LedgerBIP32Index;

const HARDENED: u32 = 0x80000000;

const BIP32_CHANGE: u32 = /*0 |*/ HARDENED;
const BIP32_INDEX: u32 = /*0 |*/ HARDENED;

pub fn get_addr_from_pubkey(pubkey: [u8; 32]) -> [u8; 33] {
    let mut hasher = VarBlake2b::new(32).unwrap();
    hasher.update(pubkey);
    let mut result: [u8; 33] = [0; 33];
    hasher.finalize_variable(|res| {
        result[1..33].clone_from_slice(&res[..32]);
    });
    result
}

pub fn bech32_from_public_key(hrp: &str, public_key: [u8; 32]) -> String {
    bech32_from_address(hrp, get_addr_from_pubkey(public_key))
}

pub fn bech32_from_address(hrp: &str, address_bytes: [u8; 33]) -> String {
    bech32::encode(hrp, address_bytes.to_base32()).unwrap()
}

pub fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new("ledger address generator")
        .version("1.0")
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

    let hrp;
    let coin;

    (hrp, coin) = match matches.value_of("coin-type") {
        Some(c) => match c {
            "iota" => ("iota", CoinType::IOTA),
            "smr" => ("smr", CoinType::Shimmer),
            "rms" => ("rms", CoinType::Testnet),
            "atoi" => ("atoi", CoinType::Testnet),
            _ => panic!("unknown coin type"),
        },
        None => ("smr", CoinType::Shimmer),
    };

    let count = match matches.value_of("number") {
        Some(c) => c.parse::<u32>().unwrap(),
        None => 5,
    };

    for n in 0..count {
        let account = n | 0x80000000;

        let ledger = iota_ledger_nano::get_ledger_by_type(coin as u32, Protocols::Nova, account, &transport_type, None)?;

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

        let public_keys = ledger.get_public_keys(false, bip32_indices, 1)?;
        let public_key_bytes = match public_keys.first() {
            Some(a) => a,
            None => panic!("no public key was generated!"),
        };
        // Convert byte array to hex string
        let public_key_hex: String = public_key_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        let mut addr_bytes_with_type = [0u8; 33];

        // first address byte is 0 for ed25519
        addr_bytes_with_type[1..33].clone_from_slice(&address_bytes[..]);

        let bech32_address = bech32_from_address(hrp, addr_bytes_with_type);
        let bech32_address_from_pubkey = bech32_from_public_key(hrp, *public_key_bytes);

        if bech32_address != bech32_address_from_pubkey {
            panic!(
                "validation failed! {} vs {}",
                bech32_address, bech32_address_from_pubkey
            )
        }

        println!(
            "validation successful! Wallet address (2c'/{:x}'/{:x}'/{:x}'/{:x}'): {} (0x{})",
            coin as i32,
            account & !HARDENED,
            BIP32_CHANGE & !HARDENED,
            BIP32_INDEX & !HARDENED,
            bech32_address,
            public_key_hex,
        );
    }

    Ok(())
}
