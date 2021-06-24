//! Library

use std::convert::TryInto;
use std::{thread, time};

//use ledger_apdu::{APDUAnswer, APDUCommand};
use ledger_transport::{errors::TransportError, Exchange};

pub mod ledger_apdu {
    pub use ledger_apdu::{APDUAnswer, APDUCommand};
}

use ledger::TransportNativeHID;
use ledger_tcp::TransportTCP;

use crate::api::constants;
use crate::api::constants::DataTypeEnum;
use crate::api::errors::APIError;

use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use trait_async::trait_async;

pub mod api;

const MINIMUM_APP_VERSION: u32 = 6002;

#[trait_async]
impl Exchange for Transport {
    async fn exchange(
        &self,
        apdu_command: &ledger_apdu::APDUCommand,
    ) -> Result<ledger_apdu::APDUAnswer, TransportError> {
        match self {
            Transport::TCP(t) => {
                return t
                    .exchange(apdu_command)
                    .await
                    .map_err(|_| TransportError::APDUExchangeError);
            }
            Transport::NativeHID(h) => {
                return h
                    .exchange(apdu_command)
                    .map_err(|_| TransportError::APDUExchangeError);
            }
            Transport::TCPWatcher(t) => {
                let apdu_answer = t
                    .transport_tcp
                    .exchange(apdu_command)
                    .await
                    .map_err(|_| TransportError::APDUExchangeError)?;
                if t.callback.is_some() {
                    (t.callback.unwrap())(apdu_command, &apdu_answer);
                }
                return Ok(apdu_answer);
            }
        }
    }
}

#[derive(Default, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct LedgerBIP32Index {
    pub bip32_index: u32,
    pub bip32_change: u32,
}

impl Packable for LedgerBIP32Index {
    fn packed_len(&self) -> usize {
        0u32.packed_len() + 0u32.packed_len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        self.bip32_index.pack(buf)?;
        self.bip32_change.pack(buf)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(Self {
            bip32_index: u32::unpack(buf)?,
            bip32_change: u32::unpack(buf)?,
        })
    }
}

type Callback = fn(apdu_command: &ledger_apdu::APDUCommand, apdu_answer: &ledger_apdu::APDUAnswer);

pub struct TransportTCPWatcher {
    callback: Option<Callback>,
    transport_tcp: Box<dyn Exchange>,
}

/// TransportTCPWatcher is a wrapper around TransportTCP
/// After data was exchanged with the underlying TCP transport, a callback is called with the APDU request and APDU response
/// The main use of the Watcher is to record test-vectors with valid responses for automatic testing.
impl TransportTCPWatcher {
    pub fn new(callback: Option<Callback>, url: &str, port: u16) -> Self {
        TransportTCPWatcher {
            callback,
            transport_tcp: Box::new(Transport::TCP(TransportTCP::new(url, port))),
        }
    }

    pub fn set_callback(&mut self, c: Callback) {
        self.callback = Some(c);
    }
}

#[derive(Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum TransportTypes {
    TCP,
    NativeHID,
    TCPWatcher,
}

#[allow(clippy::upper_case_acronyms)]
pub(crate) enum Transport {
    TCP(TransportTCP),
    NativeHID(TransportNativeHID),
    TCPWatcher(TransportTCPWatcher),
}

pub enum LedgerDeviceTypes {
    LedgerNanoX,
    LedgerNanoS,
}

pub struct LedgerHardwareWallet {
    transport: Transport,
    transport_type: TransportTypes,
    device_type: LedgerDeviceTypes,
    data_buffer_size: usize,
    is_debug_app: bool,
}

/// Get Ledger by transport_type
pub fn get_ledger_by_type(
    bip32_account: u32,
    transport_type: &TransportTypes,
    callback: Option<Callback>,
) -> Result<Box<LedgerHardwareWallet>, APIError> {
    let ledger = crate::LedgerHardwareWallet::new(transport_type, callback)?;

    // set account
    ledger.set_account(bip32_account)?;

    Ok(Box::new(ledger))
}

/// Get currently opened app
/// If "BOLOS" is returned, the dashboard is open
pub fn get_opened_app(transport_type: &TransportTypes) -> Result<(String, String), APIError> {
    let transport = crate::LedgerHardwareWallet::create_transport(transport_type, None)?;

    let app = crate::api::app_get_name::exec(&transport)?;
    Ok((app.app, app.version))
}

/// Open app on the nano s/x
/// Only works if dashboard is open
pub fn open_app(transport_type: &TransportTypes, app: String) -> Result<(), APIError> {
    let transport = crate::LedgerHardwareWallet::create_transport(transport_type, None)?;
    crate::api::app_open::exec(&transport, app)
}

/// Close current opened app on the nano s/x
/// Only works if an app is open
pub fn exit_app(transport_type: &TransportTypes) -> Result<(), APIError> {
    let transport = crate::LedgerHardwareWallet::create_transport(transport_type, None)?;
    crate::api::app_exit::exec(&transport)
}

/// Get Ledger
/// If is_simulator is true, you will get a TCP transfer for use with Speculos
/// If it's false, you will get a native USB HID transfer for real devices
pub fn get_ledger(
    bip32_account: u32,
    is_simulator: bool,
) -> Result<Box<LedgerHardwareWallet>, APIError> {
    let transport_type = match is_simulator {
        true => TransportTypes::TCP,
        false => TransportTypes::NativeHID,
    };
    get_ledger_by_type(bip32_account, &transport_type, None)
}

impl LedgerHardwareWallet {
    // only create transport without IOTA specific calls
    fn create_transport(
        transport_type: &TransportTypes,
        callback: Option<Callback>,
    ) -> Result<Transport, APIError> {
        let transport = match transport_type {
            TransportTypes::TCP => Transport::TCP(TransportTCP::new("127.0.0.1", 9999)),
            TransportTypes::NativeHID => Transport::NativeHID(
                TransportNativeHID::new().map_err(|_| APIError::TransportError)?,
            ),
            TransportTypes::TCPWatcher => {
                Transport::TCPWatcher(TransportTCPWatcher::new(callback, "127.0.0.1", 9999))
            }
        };
        Ok(transport)
    }

    // creates object but doesn't connect it
    // initialize with dummy-device
    fn new(transport_type: &TransportTypes, callback: Option<Callback>) -> Result<Self, APIError> {
        let transport = LedgerHardwareWallet::create_transport(transport_type, callback)?;

        // reset api
        crate::api::reset::exec(&transport)?;

        let res = crate::api::get_app_config::exec(&transport)?;

        let version = res.app_version_major as u32 * 1000000
            + res.app_version_minor as u32 * 1000
            + res.app_version_patch as u32;

        // signature changed from signing the essence to signing the hash of the essence (0.6.1 to 0.6.2)
        if version < MINIMUM_APP_VERSION {
            // temporary for not needing to change wallet.rs
            return Err(APIError::AppTooOld);
        }

        let device_type = match res.device {
            0 => LedgerDeviceTypes::LedgerNanoS,
            1 => LedgerDeviceTypes::LedgerNanoX,
            _ => {
                return Err(APIError::Unknown);
            }
        };

        let data_buffer_state = crate::api::get_data_buffer_state::exec(&transport)?;

        Ok(LedgerHardwareWallet {
            transport,
            transport_type: *transport_type,
            device_type,
            data_buffer_size: data_buffer_state.data_block_size as usize
                * data_buffer_state.data_block_count as usize,
            is_debug_app: res.is_debug_app == 1,
        })
    }

    fn transport(&self) -> &dyn Exchange {
        &self.transport
    }

    pub fn get_transport_type(&self) -> TransportTypes {
        self.transport_type
    }

    pub fn is_simulator(&self) -> bool {
        match self.transport {
            Transport::TCP(_) => true,
            Transport::NativeHID(_) => false,
            Transport::TCPWatcher(_) => true,
        }
    }

    // something like tri-state ... true / false / error
    pub fn is_debug_app(&self) -> bool {
        self.is_debug_app
    }

    pub fn device_type(&self) -> &LedgerDeviceTypes {
        &self.device_type
    }

    // convenience function for first getting the data buffer state and then
    // downloading as many blocks as needed from the device
    // it returns a vector with the size reported by the device
    fn read_data_bufer(&self) -> Result<Vec<u8>, APIError> {
        // get buffer state
        let dbs = api::get_data_buffer_state::exec(self.transport())?;

        // is buffer state okay? (read allowed, contains addresses, valid flag set)
        if dbs.data_type as u8 != constants::DataTypeEnum::GeneratedAddress as u8
            && dbs.data_type as u8 != constants::DataTypeEnum::Signatures as u8
        {
            return Err(APIError::CommandNotAllowed);
        }

        // buffer to read data from device
        let mut buffer: Vec<u8> = Vec::new();

        // how many block do we need to read?
        let mut blocks_needed: u8 = (dbs.data_length / dbs.data_block_size as u16) as u8;
        if (dbs.data_length % dbs.data_block_size as u16) as u8 != 0 {
            blocks_needed += 1;
        }

        // too many blocks?
        if blocks_needed > dbs.data_block_count {
            return Err(APIError::CommandInvalidData);
        }

        // read blocks to buffer
        for block in 0..blocks_needed {
            // read data buffer to get address
            let mut res = api::read_data_block::exec(self.transport(), block)?;
            buffer.append(&mut res.data);
        }
        Ok(buffer[0..dbs.data_length as usize].to_vec())
    }

    // convenience function - write as many pages as needed to transfer data to the device
    fn write_data_buffer(&self, data: Vec<u8>) -> Result<(), APIError> {
        // clear data buffer before data can be uploaded and validated
        api::clear_data_buffer::exec(self.transport())?;

        // get buffer state
        let dbs = api::get_data_buffer_state::exec(self.transport())?;

        // is buffer state okay? (write allowed, is empty)
        if dbs.data_type as u8 != DataTypeEnum::Empty as u8 {
            return Err(APIError::CommandNotAllowed);
        }

        // how many blocks to upload?
        let mut blocks_needed = (data.len() / dbs.data_block_size as usize) as u8;
        if (data.len() % dbs.data_block_size as usize) as u8 != 0 {
            blocks_needed += 1;
        }

        // too many blocks?
        if blocks_needed > dbs.data_block_count {
            return Err(APIError::CommandInvalidData);
        }

        // transfer blocks
        let mut iter = data.chunks(dbs.data_block_size as usize);
        for block in 0..blocks_needed {
            // get next chunk of data
            let mut block_data = iter.next().unwrap().to_vec();

            // block has to be exactly data_block_size but last chunk can have fewer bytes
            // fill it up to the correct size
            // TODO: is there some nicer way?
            while block_data.len() < dbs.data_block_size as usize {
                block_data.push(0u8);
            }

            // now write block
            api::write_data_block::exec(self.transport(), block as u8, block_data)?;
        }
        Ok(())
    }

    /// resets api (also resets account index)
    pub fn reset(&self) -> Result<(), APIError> {
        api::reset::exec(self.transport())?;
        Ok(())
    }

    /// Set BIP32 account index
    ///
    /// For all crypto operations following BIP32 path is used: `2c'/107a'/account'/index'`. This command sets the
    /// third component of the BIP32 path. The account index remains valid until the API is reset.
    ///
    /// The MSB (=hardened) always must be set.
    pub fn set_account(&self, bip32_account: u32) -> Result<(), APIError> {
        api::set_account::exec(self.transport(), bip32_account)?;
        Ok(())
    }

    pub fn get_addresses(
        &self,
        show: bool,
        bip32: LedgerBIP32Index,
        count: usize,
    ) -> Result<Vec<[u8; constants::ADDRESS_SIZE_BYTES]>, api::errors::APIError> {
        // if not interactive, show "generating addresses"
        if !show {
            api::show_flow::show_generating_addresses(self.transport())?;
            // give the ledger time to display the screen
            // before generating addresses
            thread::sleep(time::Duration::from_millis(250));
        }

        // clear data buffer before addresses can be generated
        api::clear_data_buffer::exec(self.transport())?;

        let max_count = self.data_buffer_size / constants::ADDRESS_WITH_TYPE_SIZE_BYTES;

        if count > max_count {
            return Err(api::errors::APIError::CommandInvalidData);
        }

        // generate one or more address(es)
        api::generate_address::exec(self.transport(), show, bip32, count as u32)?;

        // read addresses from device
        let buffer = self.read_data_bufer()?;

        let mut addresses: Vec<[u8; 32]> = Vec::new();
        for i in 0_usize..count {
            // no need to copy address type byte!
            let addr: [u8; 32] = buffer[i * constants::ADDRESS_WITH_TYPE_SIZE_BYTES + 1
                ..(i + 1) * constants::ADDRESS_WITH_TYPE_SIZE_BYTES]
                .try_into()
                .unwrap(); // each 33 bytes one address
            addresses.push(addr);
        }

        if !show {
            api::show_flow::show_main_menu(self.transport())?;
        }

        Ok(addresses)
    }

    fn use_single_sign(&self) -> Result<bool, APIError> {
        let single_sign = match self.device_type() {
            LedgerDeviceTypes::LedgerNanoS => true,
            LedgerDeviceTypes::LedgerNanoX => false,
        };
        Ok(single_sign)
    }

    /// Prepare Signing
    ///
    /// Uploads the essence, parses and validates it.
    pub fn prepare_signing(
        &self,
        key_indices: Vec<LedgerBIP32Index>,
        essence: Vec<u8>,
        has_remainder: bool,
        remainder_index: u16,
        remainder: LedgerBIP32Index,
    ) -> Result<(), api::errors::APIError> {
        // clone buffer because we have to add the key indices after the essence
        let mut buffer: Vec<u8> = essence.to_vec();
        for key in key_indices.iter() {
            key.pack(&mut buffer).map_err(|_| APIError::Unknown)?;
        }
        let buffer_len = buffer.len();

        // we can catch the error here before it happens on the hardware wallet
        // the wallet would respond with `InvalidData` but an error code indicating
        // why the data is invalid certainly is helpful.
        if buffer_len > self.data_buffer_size {
            return Err(api::errors::APIError::EssenceTooLarge);
        }

        // write data to the device
        self.write_data_buffer(buffer)?;

        let single_sign = self.use_single_sign()?;

        // now validate essence
        api::prepare_signing::exec(
            self.transport(),
            single_sign,
            has_remainder,
            remainder_index,
            remainder,
        )?;

        // get buffer state
        let dbs = api::get_data_buffer_state::exec(self.transport())?;

        // if recognized length is not the buffer_len, something went wrong
        // during parsing
        if dbs.data_length != buffer_len as u16 {
            return Err(APIError::Unknown);
        }

        Ok(())
    }

    /// User Confirm
    ///
    /// Displays the (parsed and validated) essence in human readable form on the screen of the
    /// hardware wallet and waits for accepting or rejecting it.
    pub fn user_confirm(&self) -> Result<(), APIError> {
        api::user_confirm::exec(self.transport())?;
        Ok(())
    }

    /// Sign (internal)
    ///
    /// Generates all signature in one call. Needs additional buffer space.
    /// Used for the Ledger Nano X
    fn _sign(&self) -> Result<Vec<u8>, api::errors::APIError> {
        api::show_flow::show_signing(self.transport())?;
        thread::sleep(time::Duration::from_millis(500));

        api::sign::exec(self.transport())?;

        let signatures = self.read_data_bufer()?;

        // show "signing successfully" for 500ms
        api::show_flow::show_for_ms(
            self.transport(),
            constants::Flows::FlowSignedSuccessfully,
            1500,
        )?;

        Ok(signatures)
    }

    /// Sign Single (internal)
    ///
    /// Generates one signature after the other for each input. Advante is, it doesn't need extra buffer space.
    /// Used for the Ledger Nano S
    fn _sign_single(&self, num_inputs: u16) -> Result<Vec<u8>, api::errors::APIError> {
        api::show_flow::show_signing(self.transport())?;
        thread::sleep(time::Duration::from_millis(500));

        let mut signatures: Vec<u8> = Vec::new();

        for signature_idx in 0..num_inputs as u8 {
            let mut signature = api::sign_single::exec(self.transport(), signature_idx)?;
            signatures.append(&mut signature.data);
        }

        // show "signing successfully" for 500ms
        api::show_flow::show_for_ms(
            self.transport(),
            constants::Flows::FlowSignedSuccessfully,
            1500,
        )?;

        Ok(signatures)
    }

    /// Sign
    ///
    /// The publicly usable function for signing an essence. It uses the internal functions _sign and _sign_single depending
    /// from the device. For the Nano S the _sign_single is used because it needs less RAM. For the Nano X the _sign is used
    /// because it's faster and RAM shouldn't be much of an issue.
    pub fn sign(&self, num_inputs: u16) -> Result<Vec<u8>, api::errors::APIError> {
        let single_sign = self.use_single_sign()?;
        match single_sign {
            true => self._sign_single(num_inputs),
            false => self._sign(),
        }
    }

    // methods only available if compiled with APP_DEBUG flag
    pub fn memory_dump(&self, filename: String) -> Result<(), api::errors::APIError> {
        if !self.is_debug_app() {
            return Err(APIError::CommandNotAllowed);
        }
        api::dump_memory::memory_dump(self.transport(), filename)
    }

    pub fn set_non_interactive_mode(
        &self,
        non_interactive_mode: bool,
    ) -> Result<(), api::errors::APIError> {
        if !self.is_debug_app() {
            return Err(APIError::CommandNotAllowed);
        }
        api::set_non_interactive_mode::exec(self.transport(), non_interactive_mode)
    }
}

#[cfg(test)]
mod tests {
    use bee_message::address::{Address, Ed25519Address};
    use bee_message::input::{Input, UtxoInput};
    use bee_message::output::{Output, SignatureLockedSingleOutput};

    use bee_message::payload::transaction::{Essence, RegularEssenceBuilder, TransactionId};

    use bee_common::packable::Packable;

    use std::error::Error;

    use serial_test::serial;

    const ACCOUNT: u32 = 0x80000000;

    #[derive(Debug, Clone)]
    pub struct InputIndexRecorder {
        /// the input
        pub input: Input,
        pub bech32: String,
        /// address index
        pub address_index: usize,

        pub bip32_index: crate::LedgerBIP32Index,
    }

    #[derive(Debug, Clone)]
    pub struct OutputIndexRecorder {
        pub output: Output,
        pub bech32: String,
        pub bip32_index: crate::LedgerBIP32Index,
        pub value: u64,
        pub is_remainder: bool,
    }

    fn hex(bytes: &[u8]) -> String {
        let mut ret = String::new();
        for b in bytes.iter() {
            ret.push_str(&format!("{:02x}", b));
        }
        ret
    }

    fn _build_essence_and_sign(
        is_simulator: bool,
        non_interactive: bool,
    ) -> Result<(), Box<dyn Error>> {
        let ledger = crate::get_ledger(ACCOUNT, is_simulator)?;

        if non_interactive && !ledger.is_debug_app() {
            panic!("app not compiled in is_debug_app mode");
        }

        // genesis input
        let genesis_input = Input::Utxo(
            UtxoInput::new(
                TransactionId::from([
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ]),
                0x0000,
            )
            .unwrap(),
        );

        let num_inputs = 1;

        let value_out = 1_000_000_000u64;
        let value_in = 2_779_530_283_277_761u64;

        // get first output address
        let output_addr_bytes: [u8; 32] = *ledger
            .get_addresses(
                false,
                crate::LedgerBIP32Index {
                    bip32_change: 0x80000000,
                    bip32_index: 0x80000001,
                },
                1,
            )
            .expect("error get new address")
            .first()
            .unwrap();

        let output = Output::SignatureLockedSingle(SignatureLockedSingleOutput::new(
            Address::Ed25519(Ed25519Address::new(output_addr_bytes)),
            value_out,
        )?);

        println!(
            "output-addr: {}, value: {}",
            hex(&output_addr_bytes),
            value_out
        );

        // if tests fails and the device is not the simulator, give the hint of initializing the ledger with Speculo's default words
        if !is_simulator
            && hex(&output_addr_bytes)
                != "0f58eb1351454d623a6a4366198d6cd5aa4a12a12a3caefb501476e06d8bd5b6"
        {
            println!();
            println!("when testing the app on a real device, please initialize it to use following 24 words:");
            println!("   glory promote mansion idle axis finger extra february uncover one trip resource lawn turtle enact monster seven myth punch hobby comfort wild raise skin");
            println!();
        }

        assert_eq!(
            hex(&output_addr_bytes),
            "65d7a7d80b9833ff5792038e3fb15f9906c4250f9fe3bed2d15f9ec60cec4a03"
        );

        // get new remainder address
        let remainder_bip32 = crate::LedgerBIP32Index {
            bip32_index: 0x80000002,
            bip32_change: 0x80000001,
        };
        if ledger.is_debug_app() {
            ledger.set_non_interactive_mode(non_interactive)?;
        }
        let remainder_addr_bytes = *ledger
            .get_addresses(true, remainder_bip32, 1)
            .expect("error new remainder")
            .first()
            .unwrap();

        // create output with remainder address
        let remainder = Output::SignatureLockedSingle(SignatureLockedSingleOutput::new(
            Address::Ed25519(Ed25519Address::new(remainder_addr_bytes)),
            value_in - value_out,
        )?);

        println!(
            "rem-addr: {}, value: {}",
            hex(&remainder_addr_bytes),
            value_in - value_out
        );

        assert_eq!(
            hex(&remainder_addr_bytes),
            "4d8e2ef5a76baef9f43bb86e74879d1ddbc9cb7cbbde7519d1b53f1b6b030f9c"
        );

        let mut outputs = Vec::new();
        outputs.push(output.clone());
        outputs.push(remainder.clone());

        // sort outputs
        outputs.sort_by(|a, b| a.cmp(&b));

        // add to essence
        // build essence and add input and output
        let mut essence_builder = RegularEssenceBuilder::new().add_input(genesis_input);

        // add sorted outputs
        for output in outputs {
            essence_builder = essence_builder.add_output(output.clone());
        }

        // finish essence
        let essence = essence_builder.finish().unwrap();

        // pack the essence to bytes
        let mut essence_bytes: Vec<u8> = Vec::new();

        Essence::from(essence.clone())
            .pack(&mut essence_bytes)
            .expect("error packing data");

        println!("essence: {}", hex(&essence_bytes));

        assert_eq!(hex(&essence_bytes), "0001000000000000000000000000000000000000000000000000000000000000000000000000020000004d8e2ef5a76baef9f43bb86e74879d1ddbc9cb7cbbde7519d1b53f1b6b030f9cc1939297f7df0900000065d7a7d80b9833ff5792038e3fb15f9906c4250f9fe3bed2d15f9ec60cec4a0300ca9a3b0000000000000000");

        // TODO: perhaps let it do the wallet app ...
        // after finish, search the index of the remainder output
        // because outputs are sorted lexically and index may have changed (probably)
        let mut remainder_index: u16 = 0;
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

        // bip32 index of input address
        let mut key_indices: Vec<crate::LedgerBIP32Index> = Vec::new();

        let bip32 = crate::LedgerBIP32Index {
            bip32_index: 0x80000000,
            bip32_change: 0x80000000,
        };
        key_indices.push(bip32);

        // prepare signing
        ledger
            .prepare_signing(
                key_indices,
                essence_bytes,
                true,
                remainder_index,
                remainder_bip32,
            )
            .expect("error prepare signing");

        // show essence to user
        if ledger.is_debug_app() {
            ledger.set_non_interactive_mode(non_interactive)?;
        }
        ledger.user_confirm().expect("error user confirm");

        //    ledger.memory_dump(String::from("dump_after_user_confirm.bin"))?;

        // sign
        let signature_bytes = ledger.sign(num_inputs).expect("error signing");
        println!("signature: {}", hex(&signature_bytes));

        assert_eq!(hex(&signature_bytes), "0000f9e5d9f4437cf656ef76da8fa17d38f66569ec61cca09b28d7210d0ed18b59f0d69678261469dd3b4862fb144cafa318134e1c6624912b41b170c369ff83f6d08a3ff0b04563413555f4ed8f1a0729cf05055385acaa48a9f7be71b3909f7506");

        Ok(())
    }

    #[test]
    #[serial]
    fn build_essence_and_sign_non_interactive() -> Result<(), Box<dyn Error>> {
        _build_essence_and_sign(true, true)
    }

    #[test]
    #[serial]
    fn build_essence_and_sign() -> Result<(), Box<dyn Error>> {
        _build_essence_and_sign(true, false)
    }
}
