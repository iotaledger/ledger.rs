use clap::{App, Arg};

use crypto::hashes::blake2b::{Blake2b256};
use crypto::hashes::Digest;

use iota_ledger_nano::LedgerBIP32Index;

use packable::Packable;
use bee_block::address::Address;
use bee_block::address::Ed25519Address;
use bee_block::input::{Input, UtxoInput};
use bee_block::output::{BasicOutputBuilder, InputsCommitment, Output};
use bee_block::signature::Signature::Ed25519;

use bee_block::payload::transaction::TransactionEssence;
use packable::PackableExt;

use bee_block::output::unlock_condition::{AddressUnlockCondition, UnlockCondition};

use bee_block::unlock::{Unlock};

use bee_block::payload::transaction::{
    RegularTransactionEssenceBuilder, TransactionId,
};

use std::error::Error;

fn hex(bytes: &[u8]) -> String {
    let mut ret = String::new();
    for b in bytes.iter() {
        ret.push_str(&format!("{:02x}", b));
    }
    ret
}

const HARDENED: u32 = 0x80000000;

pub fn random_essence(
    chain: u32,
    ledger: &mut iota_ledger_nano::LedgerHardwareWallet,
) -> Result<bool, Box<dyn Error>> {
    // build random config
    let num_inputs : u16 = 5;

    // build essence and add input and output
    let mut essence_builder =
        RegularTransactionEssenceBuilder::new(0u64, InputsCommitment::from([0u8; 32]));

    let account = HARDENED;

    // get new ledger object (for testing)
    ledger.set_account(chain, account)?;

    let mut key_indices: Vec<LedgerBIP32Index> = Vec::new();

    for i in 0..num_inputs as u32 {
        let input = Input::Utxo(
            UtxoInput::new(TransactionId::from([0u8;32]), i as u16).unwrap(),
        );
        essence_builder = essence_builder.add_input(input);

        let input_bip32_index = LedgerBIP32Index {
            bip32_index: i | HARDENED,
            bip32_change: HARDENED,
        };

        key_indices.push(input_bip32_index);
    }

    let output_bip32_index = LedgerBIP32Index{bip32_index: 4 | HARDENED, bip32_change: HARDENED};

    let output_addr_bytes: [u8; 32] = *ledger
        .get_addresses(false, output_bip32_index, 1)
        .expect("error get new address")
        .first()
        .unwrap();
        
    let value_out = 1337;

    let output = BasicOutputBuilder::new_with_amount(value_out)?.add_unlock_condition(
        UnlockCondition::Address(AddressUnlockCondition::new(Address::Ed25519(
            Ed25519Address::new(output_addr_bytes),
        ))),
    ).finish()?;
    essence_builder = essence_builder.add_output(Output::from(output));

    // finish essence
    let essence = essence_builder.finish().unwrap();

    // pack the essence to bytes
    let transaction_essence = TransactionEssence::from(essence);
    let essence_bytes: Vec<u8> = transaction_essence.pack_to_vec();

    println!("essence: {}", hex(&essence_bytes));

    // prepare signing in signgle-signing mode (ssm will be the default when finished)
    ledger
        .prepare_signing(
            key_indices,
            essence_bytes,
            false,
            0,
            LedgerBIP32Index::default(),
        )
        .expect("error prepare signing");

    // show essence to user
    ledger.user_confirm().expect("error user confirm");

    // sign
    let signature_bytes = ledger.sign(num_inputs).expect("error signing");

    println!();
    println!("signature: {}", hex(&signature_bytes));
    println!();

    // unpack all signatures to vector
    let mut readable = &mut &*signature_bytes;

    for _ in 0..num_inputs {
        let signature =
            Unlock::unpack::<&mut &[u8], true>(&mut readable).expect("error unpacking signature");

        match signature {
            Unlock::Signature(s) => {
                let sig = s.signature();
                match sig {
                    Ed25519(s) => {
                        let sig_address = Ed25519Address::new(Blake2b256::digest(s.public_key()).into());
                        s.is_valid(&transaction_essence.hash()[..], &sig_address)?;
                        println!("found valid signature");
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(true)
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
        .arg(
            Arg::with_name("blindsigning")
                .short("b")
                .long("blindsigning")
                .value_name("blindsigning")
                .help("use blindsigning")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("coin-type")
                .short("c")
                .long("coin-type")
                .help("select coin type (iota, smr)")
                .takes_value(true),
        )
        .get_matches();

    let is_simulator = matches.is_present("is-simulator");

    let transport_type = if is_simulator {
        iota_ledger_nano::TransportTypes::TCP
    } else {
        iota_ledger_nano::TransportTypes::NativeHID
    };

    let chain = match matches.value_of("coin-type") {
        Some(c) => match c {
            "iota" => 0x107a,
            "smr" => 0x107b,
            "rms" => 0x1,
            "atoi" => 0x1,
            _ => panic!("unknown coin type"),
        },
        None => 0x107a,
    };

    let mut ledger = iota_ledger_nano::get_ledger_by_type(
        chain,
        HARDENED,
        &transport_type,
        None
    )?;

    random_essence(chain, &mut ledger)?;

    Ok(())
}
