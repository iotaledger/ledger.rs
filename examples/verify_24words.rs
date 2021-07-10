use bech32::{self, ToBase32};
use clap::{App, Arg};

use std::error::Error;
use std::str::FromStr;

use blake2::digest::{Update, VariableOutput};
use blake2::VarBlake2b;

use iota_ledger::LedgerBIP32Index;

use bip39::Mnemonic;

use std::io::{stdin, stdout, Write};

const HARDENED: u32 = 0x80000000;

const BIP32_ACCOUNT: u32 = /*0 |*/ HARDENED;
const BIP32_CHANGE: u32 = /*0 |*/ HARDENED;
const BIP32_INDEX: u32 = /*0 |*/ HARDENED;

pub fn get_seed(words: &str, password: &str) -> [u8; 64] {
    // create a new randomly generated mnemonic phrase
    let mnemonic = match Mnemonic::parse(words) {
        Ok(b) => b,
        Err(_) => {
            panic!("parsind the 24 words failed!");
        }
    };

    // get the HD wallet seed
    mnemonic.to_seed(password)
}

pub fn get_key(
    seed: &[u8],
    chain: u32,
    account: u32,
    index: LedgerBIP32Index,
) -> Result<slip10::Key, slip10::Error> {
    let path = format!(
        "44'/{}'/{}'/{}'/{}'",
        chain,
        account & !HARDENED,
        index.bip32_change & !HARDENED,
        index.bip32_index & !HARDENED
    );
    let bip32_path = slip10::path::BIP32Path::from_str(&path)?;
    slip10::derive_key_from_path(seed, slip10::Curve::Ed25519, &bip32_path)
}
 
pub fn get_addr_from_pubkey(pubkey: [u8; 32]) -> [u8; 32] {
    let mut hasher = VarBlake2b::new(32).unwrap();
    hasher.update(pubkey);
    let mut result: [u8; 32] = [0; 32];
    hasher.finalize_variable(|res| {
        result[..32].clone_from_slice(&res[..32]);
    });
    result
}

pub fn get_addr(
    seed: &[u8],
    chain: u32,
    account: u32,
    index: LedgerBIP32Index,
) -> Result<[u8; 32], slip10::Error> {
    let key = get_key(&seed, chain, account, index)?;
    let pubkey = key.public_key();
    let mut truncated = [0u8; 32];
    truncated.clone_from_slice(&pubkey[1..33]);
    Ok(get_addr_from_pubkey(truncated))
}

pub fn get_bech32_address(hrp: &str, address_bytes: [u8; 32]) -> String {
    let mut addr_bytes_with_type = [0u8; 33];
    // first address byte is 0 for ed25519
    addr_bytes_with_type[1..33].clone_from_slice(&address_bytes[..]);
    bech32::encode(hrp, addr_bytes_with_type.to_base32()).unwrap()
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

pub fn prompt_input(prompt: &str) -> String {
    let mut s = String::new();
    print!("{}: ", prompt);
    let _ = stdout().flush();
    stdin().read_line(&mut s).expect("error entering words");
    trim_newline(&mut s);
    s
}

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
        iota_ledger::TransportTypes::TCP
    } else {
        iota_ledger::TransportTypes::NativeHID
    };

    let ledger = iota_ledger::get_ledger_by_type(BIP32_ACCOUNT, &transport_type, None)?;

    let (hrp, chain) = match !ledger.is_debug_app() {
        true => ("iota", 0x107a),
        false => ("atoi", 0x1),
    };

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

    let bech32_ledger_address = get_bech32_address(hrp, *address_bytes);

    println!();
    println!(
        "ledger-address     (2c'/{:x}'/{:x}'/{:x}'/{:x}'): {}",
        chain,
        BIP32_ACCOUNT & !HARDENED,
        BIP32_CHANGE & !HARDENED,
        BIP32_INDEX & !HARDENED,
        bech32_ledger_address
    );
    println!();
    println!("verify address above with display on the ledger nano s/x and acknowledge");
    println!();


    // generate address with prompt (to compare it)
    let _ = ledger.get_addresses(true, bip32_indices, 1)?;

    let words = prompt_input("enter your 24 words");
    let password = prompt_input("enter your passphrase");

    println!();

    //    let words = "guess egg satisfy snake narrow fiber letter lonely about twin coral width whip keep brass engine morning dress dream elbow weasel picture fork woman";
    //    let password = "";
    let seed = get_seed(words.as_str(), &password);

    let address_bytes = get_addr(&seed, chain, BIP32_ACCOUNT, bip32_indices).unwrap();
    let bech32_address = get_bech32_address(hrp, address_bytes);

    println!(
        "calculated-address (2c'/{:x}'/{:x}'/{:x}'/{:x}'): {}",
        chain,
        BIP32_ACCOUNT & !HARDENED,
        BIP32_CHANGE & !HARDENED,
        BIP32_INDEX & !HARDENED,
        bech32_address
    );

    println!();
    if bech32_ledger_address != bech32_address {
        println!();
        println!("addresses DON'T match!");
    } else {
        println!("addresses match!");
    }
    Ok(())
}
