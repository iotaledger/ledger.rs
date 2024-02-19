use bee_block::signature::Signature::Ed25519;
use bee_block::unlock::Unlock;
use clap::{App, Arg};
use crypto::signatures::ed25519;
use iota_ledger_nano::api::constants::{CoinType, Protocol};
use iota_ledger_nano::LedgerBIP32Index;
use packable::Packable;
use std::error::Error;

const HARDENED: u32 = 0x80000000;

const BIP32_CHANGE: u32 = /*0 |*/ HARDENED;
const BIP32_INDEX: u32 = /*0 |*/ HARDENED;

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

    let coin;

    (_, coin) = match matches.value_of("coin-type") {
        Some(c) => match c {
            "iota" => ("iota", CoinType::IOTA),
            "smr" => ("smr", CoinType::Shimmer),
            "rms" => ("rms", CoinType::Testnet),
            "atoi" => ("atoi", CoinType::Testnet),
            _ => panic!("unknown coin type"),
        },
        None => ("smr", CoinType::Shimmer),
    };

    let account = 0x80000000;

    let ledger =
        iota_ledger_nano::get_ledger_by_type(Protocol::Nova, coin as u32, account, &transport_type, None)?;

    let bip32_index = LedgerBIP32Index {
        bip32_change: BIP32_CHANGE,
        bip32_index: BIP32_INDEX,
    };
    let bip32_indices = [bip32_index];
    let signing_input_hex_32bytes =
        "351acfc38480083ca4855d832f662b4f00d26fc875e8477924a4678e7f7a3c32";
    let signing_input_hex_64bytes = "351acfc38480083ca4855d832f662b4f00d26fc875e8477924a4678e7f7a3c3223fa8d3d9e3fd76795d069bf2b79e55ec7ab9cf309727cac446910ccebf78b3a";

    for i in 1..2 {
        let signing_input = hex::decode(if i == 0 {
            signing_input_hex_32bytes
        } else {
            signing_input_hex_64bytes
        })?;

        ledger.prepare_blind_signing(bip32_indices.to_vec(), signing_input.to_vec())?;

        ledger.user_confirm().expect("error user confirm");

        let signature_bytes = ledger.sign(1).expect("error signing");
        let mut readable = &mut &*signature_bytes;

        println!("signature: {}", hex::encode(&signature_bytes));
        println!();

        let signature =
            Unlock::unpack::<&mut &[u8], true>(&mut readable).expect("error unpacking signature");

        match signature {
            Unlock::Signature(s) => {
                let sig = s.signature();
                match sig {
                    Ed25519(s) => {
                        let sig: [u8; 64] = *s.signature();
                        let sig = ed25519::Signature::from_bytes(sig);

                        let pub_key_bytes = s.public_key();

                        let pub_key = ed25519::PublicKey::try_from_bytes(*pub_key_bytes).unwrap();

                        if !pub_key.verify(&sig, &signing_input) {
                            panic!("error verifying signature");
                        }

                        println!("found valid signature");
                    }
                }
            }
            _ => panic!("blindsigning only should generate signature unlocks"),
        };
    }

    Ok(())
}
