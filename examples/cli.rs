use bech32::{self, ToBase32};
use clap::{App, Arg};
use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use std::io::prelude::*;
use std::sync::Mutex;

use std::convert::TryInto;

use bip39::Mnemonic;

use std::collections::HashMap;

use blake2::digest::{Update, VariableOutput};
use blake2::VarBlake2b;

use iota_ledger::LedgerBIP32Index;

use bee_message::address::{Address, Ed25519Address};
use bee_message::input::{Input, UtxoInput};
use bee_message::output::{Output, SignatureLockedSingleOutput};
use bee_message::signature::{Ed25519Signature, SignatureUnlock};
use bee_message::unlock::{ReferenceUnlock, UnlockBlock};

use bee_message::payload::transaction::{
    Essence, RegularEssence, RegularEssenceBuilder, TransactionId,
};

use bee_common::packable::Packable;

use iota_ledger::ledger_apdu::{APDUAnswer, APDUCommand};

use crypto::signatures::ed25519;

use std::error::Error;

use std::sync::atomic::{AtomicBool, Ordering};

use std::fs::File;

lazy_static! {
    static ref WRITER: Mutex<WriterWrapper> = Mutex::new(WriterWrapper::default());
}

enum FormatTypes {
    JSON,
    HEX,
    BIN,
}

#[derive(Default)]
struct WriterWrapper {
    writer: Option<File>,
    format: Option<FormatTypes>,
}

impl WriterWrapper {
    pub fn open(&mut self, filename: String) -> std::io::Result<()> {
        self.writer = Some(File::create(filename)?);
        Ok(())
    }
    pub fn write(&mut self, s: &str) {
        let file = self.writer.as_mut().unwrap();
        writeln!(file, "{}", s).expect("error writing file");
    }
    pub fn write_bin(&mut self, b: &[u8]) {
        let file = self.writer.as_mut().unwrap();
        file.write_all(b).expect("error writing file");
    }

    pub fn set_format_type(&mut self, format_type: FormatTypes) {
        self.format = Some(format_type);
    }
}

// deterministic tests are important for finding errors!
const PRNG_SEED: [u8; 32] = [
    0xfc, 0x11, 0xb4, 0xdd, 0x28, 0x74, 0x1c, 0x15, 0xcc, 0xac, 0x4b, 0x26, 0xaf, 0x43, 0x06, 0x84,
    0xc8, 0x04, 0x55, 0x56, 0x3a, 0xda, 0xea, 0x1d, 0x80, 0x21, 0xd9, 0xbf, 0x6e, 0x5b, 0x25, 0x22,
];

const DEFAULT_WORDS : &str = "glory promote mansion idle axis finger extra february uncover one trip resource lawn turtle enact monster seven myth punch hobby comfort wild raise skin";

const DEFAULT_SEED: [u8; 64] = [
    0xb1, 0x19, 0x97, 0xfa, 0xff, 0x42, 0x0a, 0x33, 0x1b, 0xb4, 0xa4, 0xff, 0xdc, 0x8b, 0xdc, 0x8b,
    0xa7, 0xc0, 0x17, 0x32, 0xa9, 0x9a, 0x30, 0xd8, 0x3d, 0xbb, 0xeb, 0xd4, 0x69, 0x66, 0x6c, 0x84,
    0xb4, 0x7d, 0x09, 0xd3, 0xf5, 0xf4, 0x72, 0xb3, 0xb9, 0x38, 0x4a, 0xc6, 0x34, 0xbe, 0xba, 0x2a,
    0x44, 0x0b, 0xa3, 0x6e, 0xc7, 0x66, 0x11, 0x44, 0x13, 0x2f, 0x35, 0xe2, 0x06, 0x87, 0x35, 0x64,
];
const DEFAULT_KEY_DEBUG: &str = "171167b16cb8dcfa0b4f46e9bbb196cfbb2ee9b5ba7d9f19786ac6974ece46d1";

const DEFAULT_KEY: &str = "f14f5bc7f78179df26fed411de31e6e1344f272597972bc975cedff700819d95";

// output-range to test address pooling
const MAX_INPUT_RANGE: u32 = 100;
const MAX_OUTPUT_RANGE: u32 = 100;
const MAX_REMAINDER_RANGE: u32 = 100;

const MAX_ACCOUNT_RANGE: u32 = 4;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

fn hex(bytes: &[u8]) -> String {
    let mut ret = String::new();
    for b in bytes.iter() {
        ret.push_str(&format!("{:02x}", b));
    }
    ret
}

lazy_static! {
    static ref DEBUG_APP: AtomicBool = AtomicBool::new(false);
}

const HARDENED: u32 = 0x80000000;
const MAX_INPUTS: u16 = 126;
const MAX_OUTPUTS: u16 = 126;

// nano s
const MAX_SUM_INPUTS_OUTPUTS: u16 = 16;

use anyhow::Result;

pub fn get_seed() -> [u8; 64] {
    // create a new randomly generated mnemonic phrase
    let mnemonic = match Mnemonic::parse(DEFAULT_WORDS) {
        Ok(b) => b,
        Err(e) => {
            panic!("e: {}", e);
        }
    };

    // get the HD wallet seed
    mnemonic.to_seed("")
}
pub fn get_key(
    seed: &[u8],
    account: u32,
    index: LedgerBIP32Index,
) -> Result<slip10::Key, slip10::Error> {
    let is_debug_app = DEBUG_APP.load(Ordering::Relaxed);

    let path = format!(
        "44'/{}'/{}'/{}'/{}'",
        if !is_debug_app { 0x107a } else { 0x1 },
        account & !0x80000000,
        index.bip32_change & !0x80000000,
        index.bip32_index & !0x80000000
    );
    let bip32_path = slip10::path::BIP32Path::from_str(&path)?;
    slip10::derive_key_from_path(&seed[..], slip10::Curve::Ed25519, &bip32_path)
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
    account: u32,
    index: LedgerBIP32Index,
) -> Result<[u8; 32], slip10::Error> {
    let key = get_key(&seed, account, index)?;
    let pubkey = key.public_key();
    let mut truncated = [0u8; 32];
    truncated.clone_from_slice(&pubkey[1..33]);
    Ok(get_addr_from_pubkey(truncated))
}

/// A record matching an Input with its address.
#[derive(Debug, Clone)]
pub struct InputIndexRecorder {
    /// the input
    pub input: Input,
    pub bech32: String,
    /// address index
    pub address_index: usize,

    pub bip32_index: LedgerBIP32Index,
}

#[derive(Debug, Clone)]
pub struct OutputIndexRecorder {
    pub output: Output,
    pub bech32: String,
    pub bip32_index: LedgerBIP32Index,
    pub value: u64,
    pub is_remainder: bool,
}

/// Gets the unlock blocks for a transaction.
pub fn get_transaction_unlock_blocks(
    account: u32,
    essence: &RegularEssence,
    address_index_recorders: &mut [InputIndexRecorder],
) -> Result<Vec<UnlockBlock>> {
    let mut serialized_essence = Vec::new();
    Essence::from(essence.clone())
        .pack(&mut serialized_essence)
        .map_err(|_| anyhow::anyhow!("invalid parameter: inputs"))?;

    let mut hasher = VarBlake2b::new(32).unwrap();
    hasher.update(serialized_essence);
    let mut hashed_essence: [u8; 32] = [0; 32];
    hasher.finalize_variable(|res| {
        hashed_essence[..32].clone_from_slice(&res[..32]);
    });

    let seed = get_seed();
    let mut unlock_blocks = vec![];
    let mut signature_indexes = HashMap::<LedgerBIP32Index, usize>::new();
    address_index_recorders.sort_by(|a, b| a.input.cmp(&b.input));
    for (current_block_index, recorder) in address_index_recorders.iter().enumerate() {
        // Check if current path is same as previous path
        // If so, add a reference unlock block

        if let Some((_, v)) = signature_indexes.iter().find_map(|(k, v)| {
            if k == &recorder.bip32_index {
                Some((k, v))
            } else {
                None
            }
        }) {
            log::info!(
                "found reference {:08x}:{:08x} {} -> {}",
                recorder.bip32_index.bip32_change,
                recorder.bip32_index.bip32_index,
                current_block_index,
                *v
            );
            unlock_blocks.push(UnlockBlock::Reference(
                ReferenceUnlock::new(*v as u16)
                    .map_err(|_| anyhow::anyhow!("failed to create reference unlock block"))?,
            ));
        } else {
            // If not, we should create a signature unlock block
            let private_key = get_key(&seed, account, recorder.bip32_index).unwrap();

            let iota_priv_key = ed25519::SecretKey::from_le_bytes(private_key.key).unwrap();

            let public_key = private_key.public_key();
            let mut public_key_trunc = [0u8; 32];

            public_key_trunc.clone_from_slice(&public_key[1..33]);

            // The block should sign the entire transaction essence part of the transaction payload
            let signature = Box::new(iota_priv_key.sign(&hashed_essence).to_bytes());
            unlock_blocks.push(UnlockBlock::Signature(SignatureUnlock::Ed25519(
                Ed25519Signature::new(public_key_trunc, *signature),
            )));
            signature_indexes.insert(recorder.bip32_index, current_block_index);
            log::info!(
                "put {:08x}:{:08x} {} into signatures_indexes",
                recorder.bip32_index.bip32_change,
                recorder.bip32_index.bip32_index,
                current_block_index
            );
            // Update current block index
        }
    }
    Ok(unlock_blocks)
}

pub fn random_essence(
    ledger: &mut iota_ledger::LedgerHardwareWallet,
    seed: &[u8],
    rnd: &mut SmallRng,
    non_interactive: bool,
) -> Result<bool, Box<dyn Error>> {
    // build random config
    let num_inputs = rnd.next_u32() as u16 % MAX_INPUTS + 1;
    let num_outputs = rnd.next_u32() as u16 % MAX_OUTPUTS + 1;

    if num_inputs + num_outputs > MAX_SUM_INPUTS_OUTPUTS {
        return Ok(false);
    }

    let has_remainder = rnd.next_u32() & 0x1 != 0;
    let mut remainder_index: u16 = 0;

    let config = format!(
        "{}|{}|{}|{}",
        num_inputs,
        num_outputs,
        if has_remainder { 1 } else { 0 },
        remainder_index
    );

    let mut hm = HASHMAP.lock().unwrap();

    let is_new = match hm.get(&config) {
        Some(_) => false,
        None => {
            hm.insert(config.clone(), true);
            true
        }
    };
    drop(hm);
    if !is_new {
        return Ok(false);
    }

    // add to essence
    // build essence and add input and output
    let mut essence_builder = RegularEssenceBuilder::new();

    #[allow(clippy::modulo_one)]
    let account = (rnd.next_u32() % MAX_ACCOUNT_RANGE) | HARDENED;
    println!("account: 0x{:08x}", account & !0x80000000);

    // get new ledger object (for testing)
    ledger.set_account(account)?;

    let hrp: &str = if !ledger.is_debug_app() {
        "iota"
    } else {
        "atoi"
    };

    let mut address_index_recorders: Vec<InputIndexRecorder> = Vec::new();

    let mut key_indices: Vec<LedgerBIP32Index> = Vec::new();
    let mut key_strings: Vec<String> = Vec::new();

    for i in 0..num_inputs {
        let mut txid = [0u8; 32];
        rnd.fill_bytes(&mut txid);

        let input = Input::Utxo(
            UtxoInput::new(TransactionId::from(txid), rnd.next_u32() as u16 % 127).unwrap(),
        );

        let is_change = rnd.next_u32() & 0x1 == 0x1;
        let input_bip32_index = LedgerBIP32Index {
            bip32_index: (rnd.next_u32() % MAX_INPUT_RANGE) | HARDENED,
            bip32_change: if is_change { 1 } else { 0 } | HARDENED,
        };

        //        println!("index: {:08x}{:08x}", input_bip32_index.bip32_change, input_bip32_index.bip32_index);

        let input_addr_bytes: [u8; 32] = *ledger
            .get_addresses(false, input_bip32_index, 1)
            .expect("error get new address")
            .first()
            .unwrap();

        let mut addr_bytes_with_type = [0u8; 33];
        addr_bytes_with_type[0] = 0; // ed25519
        addr_bytes_with_type[1..33].clone_from_slice(&input_addr_bytes[..]);
        let b32 = bech32::encode(hrp, addr_bytes_with_type.to_base32()).unwrap();

        //        println!("input: {}", b32);

        key_indices.push(input_bip32_index);
        key_strings.push(b32.clone());

        address_index_recorders.push(InputIndexRecorder {
            address_index: i as usize,
            bip32_index: input_bip32_index,
            input,
            bech32: b32.clone(),
        });
    }

    let mut output_recorder: Vec<OutputIndexRecorder> = Vec::new();
    for _ in 0..num_outputs {
        let mut output_bip32_index = None;
        loop {
            if output_bip32_index.is_some()
                && !output_recorder
                    .clone()
                    .into_iter()
                    .any(|a| a.bip32_index == output_bip32_index.unwrap())
            {
                break;
            }
            output_bip32_index = Some(
                LedgerBIP32Index{bip32_index: (rnd.next_u32() % MAX_OUTPUT_RANGE) | HARDENED, bip32_change: /* 0 | */ HARDENED},
            );
        }
        let output_bip32_index = output_bip32_index.unwrap();

        let output_addr_bytes: [u8; 32] = *ledger
            .get_addresses(false, output_bip32_index, 1)
            .expect("error get new address")
            .first()
            .unwrap();
        let value_out = rnd.next_u32() as u64 + 1u64;
        let output = Output::SignatureLockedSingle(SignatureLockedSingleOutput::new(
            Address::Ed25519(Ed25519Address::new(output_addr_bytes)),
            value_out,
        )?);

        //essence_builder = essence_builder.add_output(output.clone());

        let mut addr_bytes_with_type = [0u8; 33];
        addr_bytes_with_type[0] = 0; // ED25519
        addr_bytes_with_type[1..33].clone_from_slice(&output_addr_bytes[..]);
        let b32 = bech32::encode(hrp, addr_bytes_with_type.to_base32()).unwrap();

        let cmp_addr = get_addr(&seed, account, output_bip32_index).unwrap();
        addr_bytes_with_type[1..33].clone_from_slice(&cmp_addr[..]);
        let cmp_b32 = bech32::encode(hrp, addr_bytes_with_type.to_base32()).unwrap();

        output_recorder.push(OutputIndexRecorder {
            output: output.clone(),
            bech32: b32.clone(),
            bip32_index: output_bip32_index,
            value: value_out,
            is_remainder: false,
        });

        //        println!("output: {} {}", b32, value_out);

        assert_eq!(b32, cmp_b32);
    }

    let mut remainder_bip32 = LedgerBIP32Index::default();
    let mut remainder_addr_bytes = [0u8; 32];
    //    let mut remainder_recorer : Vec<OutputIndexRecorder> = Vec::new();
    if has_remainder {
        if ledger.is_debug_app() {
            ledger.set_non_interactive_mode(non_interactive)?;
        }
        let mut bip32 = None;
        loop {
            if bip32.is_some()
                && !output_recorder
                    .clone()
                    .into_iter()
                    .any(|a| a.bip32_index == bip32.unwrap())
            {
                break;
            }
            bip32 = Some(LedgerBIP32Index {
                bip32_index: (rnd.next_u32() % MAX_REMAINDER_RANGE) | HARDENED,
                bip32_change: 1 | HARDENED,
            });
        }
        remainder_bip32 = bip32.unwrap();

        remainder_addr_bytes = *ledger
            .get_addresses(true, remainder_bip32, 1)
            .expect("error new remainder")
            .first()
            .unwrap();
        let value_remainder = rnd.next_u32() as u64;

        // create output with remainder address
        let remainder = Output::SignatureLockedSingle(SignatureLockedSingleOutput::new(
            Address::Ed25519(Ed25519Address::new(remainder_addr_bytes)),
            value_remainder,
        )?);
        //essence_builder = essence_builder.add_output(remainder.clone());

        let mut addr_bytes_with_type = [0u8; 33];
        addr_bytes_with_type[0] = 0; // ed25519
        addr_bytes_with_type[1..33].clone_from_slice(&remainder_addr_bytes[..32]);

        let b32 = bech32::encode(hrp, addr_bytes_with_type.to_base32()).unwrap();

        output_recorder.push(OutputIndexRecorder {
            output: remainder,
            bech32: b32.clone(),
            bip32_index: remainder_bip32,
            value: value_remainder,
            is_remainder: true,
        });

        let cmp_addr = get_addr(&seed, account, remainder_bip32).unwrap();
        addr_bytes_with_type[1..33].clone_from_slice(&cmp_addr[..]);
        let cmp_b32 = bech32::encode(hrp, addr_bytes_with_type.to_base32()).unwrap();

        /*
                println!(
                    "remainder: {} {} {:08x}{:08x} {}",
                    b32,
                    remainder_index,
                    remainder_bip32.bip32_change & !0x80000000,
                    remainder_bip32.bip32_index & !0x80000000,
                    value_remainder
                );
        */
        assert_eq!(b32, cmp_b32);
    }

    output_recorder.sort_by(|a, b| a.output.cmp(&b.output));
    address_index_recorders.sort_by(|a, b| a.input.cmp(&b.input));

    // sort inputs
    for recorder in address_index_recorders.clone() {
        essence_builder = essence_builder.add_input(recorder.input.clone());
    }

    // sort outputs
    for recorder in output_recorder.clone() {
        essence_builder = essence_builder.add_output(recorder.output.clone());
    }

    // finish essence
    let essence = essence_builder.finish().unwrap();

    // swap the remainder - mainly for displaying the remainder as last output later
    if has_remainder {
        let mut or_rem_idx = 0;
        let l = output_recorder.len();
        for (i, item) in output_recorder.iter().enumerate() {
            if item.is_remainder {
                or_rem_idx = i;
            }
        }
        output_recorder.swap(or_rem_idx, l - 1);
    }

    // pack the essence to bytes
    let mut essence_bytes: Vec<u8> = Vec::new();
    Essence::from(essence.clone())
        .pack(&mut essence_bytes)
        .expect("error packing data");

    println!("essence: {}", hex(&essence_bytes));

    if has_remainder {
        // after finish, search the index of the remainder output
        // because outputs are sorted lexically and index may have changed (probably)
        for output in essence.outputs().iter() {
            match output {
                Output::SignatureLockedSingle(s) => {
                    let remainder_output =
                        Address::Ed25519(Ed25519Address::new(remainder_addr_bytes));
                    if remainder_output == *s.address() {
                        println!("found at {}", remainder_index);
                        break;
                    }
                }
                _ => {
                    panic!("unknown output type");
                }
            }
            remainder_index += 1;
        }

        // was index found?
        if remainder_index as usize == essence.outputs().len() {
            panic!("index was not found");
        }
    }

    println!("new configuration: {}", config);

    // Gets the unlock blocks for a transaction.
    let ref_blocks =
        get_transaction_unlock_blocks(account, &essence, &mut address_index_recorders).unwrap();

    let mut key_indices: Vec<LedgerBIP32Index> = Vec::new();
    let mut key_strings_new: Vec<String> = Vec::new();
    println!();
    for m in address_index_recorders {
        //        println!("index: {:08x}{:08x}", m.bip32_index.bip32_change, m.bip32_index.bip32_index);
        let path = format!(
            "2c'/{:x}'/{:x}'/{:x}'/{:x}'",
            if !ledger.is_debug_app() { 0x107a } else { 0x1 },
            account & !0x80000000,
            m.bip32_index.bip32_change & !0x80000000,
            m.bip32_index.bip32_index & !0x80000000
        );
        println!("input: {} {}", m.bech32, path);

        key_indices.push(m.bip32_index);
        let bsx = (*key_strings.get(m.address_index).unwrap()).clone();
        //        println!("input: {}", bsx);
        key_strings_new.push(bsx);
    }
    let key_strings = key_strings_new;
    println!();
    for m in output_recorder {
        if !m.is_remainder {
            println!("output: {} {}", m.bech32, m.value);
        } else {
            let path = format!(
                "2c'/{:x}'/{:x}'/{:x}'/{:x}'",
                if !ledger.is_debug_app() { 0x107a } else { 0x1 },
                account & !0x80000000,
                m.bip32_index.bip32_change & !0x80000000,
                m.bip32_index.bip32_index & !0x80000000
            );
            println!();
            println!("remainder: {} {} {}", m.bech32, path, m.value);
        }
    }

    /*for i in 0_usize..num_inputs as usize {
        println!("input indices: {:08x} vs {:08x}", *key_indices.get(i).unwrap(), (*address_index_recorders.get(i).unwrap()).bip32_index | HARDENED)
    }*/

    // prepare signing in signgle-signing mode (ssm will be the default when finished)
    ledger
        .prepare_signing(
            key_indices,
            essence_bytes.clone(),
            has_remainder,
            remainder_index,
            remainder_bip32,
        )
        .expect("error prepare signing");

    // show essence to user
    if ledger.is_debug_app() {
        ledger.set_non_interactive_mode(non_interactive)?;
    }
    ledger.user_confirm().expect("error user confirm");

    println!();
    // sign
    let signature_bytes = ledger.sign(num_inputs).expect("error signing");
    println!("signature: {}", hex(&signature_bytes));
    println!();

    let mut hasher = VarBlake2b::new(32).unwrap();
    hasher.update(essence_bytes.clone());
    let mut hashed_essence: [u8; 32] = [0; 32];
    hasher.finalize_variable(|res| {
        hashed_essence[..32].clone_from_slice(&res[..32]);
    });

    // unpack all signatures to vector
    let mut readable = &mut &*signature_bytes;
    for t in 0..num_inputs {
        let signature = UnlockBlock::unpack(&mut readable).expect("error unpacking signature");

        let signature2 = ref_blocks.get(t as usize).unwrap();

        assert_eq!(&signature, signature2);

        match signature {
            UnlockBlock::Signature(s) => {
                match s {
                    SignatureUnlock::Ed25519(s) => {
                        let sig: [u8; 64] = s.signature().try_into()?;
                        let sig = ed25519::Signature::from_bytes(sig);

                        let pub_key_bytes = s.public_key();

                        let addr_bytes = get_addr_from_pubkey(*pub_key_bytes);
                        let mut addr_bytes_with_type = [0u8; 33];
                        addr_bytes_with_type[0] = 0; // ed25519
                        addr_bytes_with_type[1..33].clone_from_slice(&addr_bytes[..]);
                        let b32 = bech32::encode(hrp, addr_bytes_with_type.to_base32()).unwrap();
                        //                        println!("{} vs {}", b32, key_strings.get(t as usize).unwrap());

                        assert_eq!(b32, *key_strings.get(t as usize).unwrap());

                        let pub_key =
                            ed25519::PublicKey::from_compressed_bytes(*pub_key_bytes).unwrap();

                        if !pub_key.verify(&sig, &hashed_essence) {
                            panic!("error verifying signature");
                        }

                        println!("found valid signature");
                    }
                    _ => {
                        panic!("unsupported signature");
                    }
                }
            }
            UnlockBlock::Reference(_) => {
                // NOP
                println!("found reference");
            }
            _ => {
                panic!("unsupported signature");
            }
        }
    }

    Ok(true)
}

fn watcher_cb(apdu_command: &APDUCommand, apdu_answer: &APDUAnswer) {
    let mut writer = WRITER.lock().unwrap();

    let raw_command = apdu_command.serialize();
    let mut raw_answer: Vec<u8> = Vec::new();
    raw_answer.extend(&apdu_answer.data);
    raw_answer.extend(&apdu_answer.retcode.to_be_bytes());

    match writer.format {
        Some(FormatTypes::JSON) => {
            let command_data: String = apdu_command
                .data
                .iter()
                .map(|b| format!("{}", b))
                .collect::<Vec<String>>()
                .join(", ");
            let answer_data: String = apdu_answer
                .data
                .iter()
                .map(|b| format!("{}", b))
                .collect::<Vec<String>>()
                .join(", ");
            let command = format!(
                "{{\"cla\":{}, \"ins\":{}, \"p1\":{}, \"p2\":{}, \"data\":[{}]}}",
                apdu_command.cla, apdu_command.ins, apdu_command.p1, apdu_command.p2, command_data
            );
            let answer = format!(
                "{{\"data\":[{}], \"retcode\":{}}}",
                answer_data, apdu_answer.retcode
            );
            writer.write(&command);
            writer.write(&answer);
        }
        Some(FormatTypes::HEX) => {
            writer.write(&format!(">>{}", hex(&raw_command)));
            writer.write(&format!("<<{}", hex(&raw_answer)));
        }
        Some(FormatTypes::BIN) => {
            let rcl = raw_command.len() as u32;
            let ral = raw_answer.len() as u32;
            writer.write_bin(&rcl.to_le_bytes());
            writer.write_bin(&raw_command);
            writer.write_bin(&ral.to_le_bytes());
            writer.write_bin(&raw_answer);
        }
        None => {
            panic!("no output format set");
        }
    }
    drop(writer);
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
            Arg::with_name("non-interactive")
                .short("n")
                .long("non-interactive")
                .value_name("non_interactive")
                .help("run the program in non-interactive mode for automatic testing")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("recorder")
                .short("r")
                .long("recorder")
                .help("record APDU requests and responses to a file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .help("user output format hex, bin, json (default) as output file format")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dump")
                .short("d")
                .long("dump")
                .help("dump memory after tests")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("limit")
                .short("l")
                .long("limit")
                .help("maximum number of tests done")
                .takes_value(true),
        )
        .get_matches();

    let is_simulator = matches.is_present("is-simulator");

    let non_interactive = matches.is_present("non-interactive");

    let limit = match matches.is_present("limit") {
        true => matches.value_of("limit").unwrap().parse::<u32>().unwrap(),
        false => 0,
    };

    let transport_type = if matches.is_present("recorder") {
        if !is_simulator {
            panic!("transport watcher only is supported for the simulator");
        }
        let filename = matches.value_of("recorder");
        if filename.is_none() {
            panic!("no filename provided!");
        }

        let mut writer = WRITER.lock().unwrap();

        let format_type = match matches.value_of("format") {
            Some(f) => match f {
                "hex" => FormatTypes::HEX,
                "json" => FormatTypes::JSON,
                "bin" => FormatTypes::BIN,
                _ => panic!("unknown format"),
            },
            None => FormatTypes::JSON,
        };

        writer.set_format_type(format_type);
        writer.open(String::from(filename.unwrap()))?;
        drop(writer);
        iota_ledger::TransportTypes::TCPWatcher
    } else {
        if is_simulator {
            iota_ledger::TransportTypes::TCP
        } else {
            iota_ledger::TransportTypes::NativeHID
        }
    };

    println!("{} {}", is_simulator, non_interactive);

    let mut rnd = SmallRng::from_seed(PRNG_SEED);

    let seed = get_seed();

    assert_eq!(DEFAULT_SEED, &seed[..]);

    let mut ledger =
        iota_ledger::get_ledger_by_type(0x80000000, &transport_type, Some(watcher_cb))?;

    let is_debug_app = ledger.is_debug_app();
    DEBUG_APP.store(is_debug_app, Ordering::Release);

    let path = format!(
        "44'/{}'/{}'/{}'/{}'",
        if !is_debug_app { 0x107a } else { 0x1 },
        0,
        0,
        0
    );
    println!("path: {}", path);
    let bip32_path = slip10::path::BIP32Path::from_str(&path).unwrap();
    let key = slip10::derive_key_from_path(&seed[..], slip10::Curve::Ed25519, &bip32_path).unwrap();

    if is_debug_app {
        assert_eq!(DEFAULT_KEY_DEBUG, hex(&key.key));
    } else {
        assert_eq!(DEFAULT_KEY, hex(&key.key));
    }

    let mut run: u32 = 0;
    for _ in 0..10000 {
        let was_new = random_essence(&mut ledger, &seed, &mut rnd, non_interactive)?;
        if was_new {
            run += 1;
            println!("\nrun {} successful\n", run);
            if limit == run {
                println!("limit reached.");
                break;
            }
        }
    }

    if matches.is_present("dump") {
        ledger.memory_dump(String::from("dump_after_test.bin"))?;
    }

    Ok(())
}
