//! Library

use std::convert::TryInto;

pub mod ledger;
pub use ledger::ledger_apdu::{APDUAnswer, APDUCommand};

use crate::api::constants;
use crate::api::constants::DataTypeEnum;
use crate::api::errors::APIError;

pub use crate::ledger::ledger_transport_tcp::Callback;
pub use crate::transport::{LedgerTransport, Transport, TransportTypes};

pub use crate::api::packable::{Error as PackableError, Packable, Read, Write};

pub mod api;
pub mod transport;

const MINIMUM_APP_VERSION: u32 = 6002;
const MINIMUM_APP_VERSION_GENERATE_PUBLIC_KEYS: u32 = 8007; // generate public keys supported starting with 0.8.7

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

pub enum LedgerDeviceTypes {
    LedgerNanoS,
    LedgerNanoSPlus,
    LedgerNanoX,
}

pub struct LedgerHardwareWallet {
    version: u32,
    transport: Transport,
    transport_type: TransportTypes,
    device_type: LedgerDeviceTypes,
    data_buffer_size: usize,
    is_debug_app: bool,
}

/// Get Ledger by transport_type
pub fn get_ledger_by_type(
    coin_type: u32,
    bip32_account: u32,
    transport_type: &TransportTypes,
    callback: Option<crate::ledger::ledger_transport_tcp::Callback>,
) -> Result<Box<LedgerHardwareWallet>, APIError> {
    let ledger = crate::LedgerHardwareWallet::new(transport_type, callback)?;

    // set account
    ledger.set_account(coin_type, bip32_account)?;

    Ok(Box::new(ledger))
}

/// Get currently opened app
/// If "BOLOS" is returned, the dashboard is open
pub fn get_opened_app(transport_type: &TransportTypes) -> Result<(String, String), APIError> {
    let transport = crate::transport::create_transport(transport_type, None)?;

    let app = crate::api::app_get_name::exec(&transport)?;
    Ok((app.app, app.version))
}

pub fn get_app_config(
    transport_type: &TransportTypes,
) -> Result<api::get_app_config::Response, APIError> {
    let transport = crate::transport::create_transport(transport_type, None)?;

    let app_config = crate::api::get_app_config::exec(&transport)?;

    Ok(app_config)
}

pub fn get_buffer_size(transport_type: &TransportTypes) -> Result<usize, APIError> {
    let transport = crate::transport::create_transport(transport_type, None)?;

    let data_buffer_state = crate::api::get_data_buffer_state::exec(&transport)?;

    Ok(data_buffer_state.data_block_size as usize * data_buffer_state.data_block_count as usize)
}

/// Open app on the nano s/x
/// Only works if dashboard is open
pub fn open_app(transport_type: &TransportTypes, app: String) -> Result<(), APIError> {
    let transport = crate::transport::create_transport(transport_type, None)?;
    crate::api::app_open::exec(&transport, app)
}

/// Close current opened app on the nano s/x
/// Only works if an app is open
pub fn exit_app(transport_type: &TransportTypes) -> Result<(), APIError> {
    let transport = crate::transport::create_transport(transport_type, None)?;
    crate::api::app_exit::exec(&transport)
}

/// Get Ledger
/// If is_simulator is true, you will get a TCP transfer for use with Speculos
/// If it's false, you will get a native USB HID transfer for real devices
pub fn get_ledger(
    coin_type: u32,
    bip32_account: u32,
    is_simulator: bool,
) -> Result<Box<LedgerHardwareWallet>, APIError> {
    let transport_type = match is_simulator {
        true => TransportTypes::TCP,
        false => TransportTypes::NativeHID,
    };
    get_ledger_by_type(coin_type, bip32_account, &transport_type, None)
}

impl LedgerHardwareWallet {
    // creates object but doesn't connect it
    // initialize with dummy-device
    fn new(transport_type: &TransportTypes, callback: Option<Callback>) -> Result<Self, APIError> {
        let transport = crate::transport::create_transport(transport_type, callback)?;

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
            2 => LedgerDeviceTypes::LedgerNanoSPlus,
            _ => {
                return Err(APIError::Unknown);
            }
        };

        let data_buffer_state = crate::api::get_data_buffer_state::exec(&transport)?;

        Ok(LedgerHardwareWallet {
            version,
            transport,
            transport_type: *transport_type,
            device_type,
            data_buffer_size: data_buffer_state.data_block_size as usize
                * data_buffer_state.data_block_count as usize,
            is_debug_app: res.is_debug_app == 1,
        })
    }

    fn transport(&self) -> &Transport {
        &self.transport
    }

    pub fn get_transport_type(&self) -> TransportTypes {
        self.transport_type
    }

    pub fn is_simulator(&self) -> bool {
        match self.transport.transport {
            LedgerTransport::TCP(_) => true,
            LedgerTransport::NativeHID(_) => false,
        }
    }

    // something like tri-state ... true / false / error
    pub fn is_debug_app(&self) -> bool {
        self.is_debug_app
    }

    pub fn device_type(&self) -> &LedgerDeviceTypes {
        &self.device_type
    }

    pub fn get_buffer_size(&self) -> usize {
        self.data_buffer_size
    }

    // uses the get_data_buffer_state-Api call to figure out if the ledger is locked
    pub fn is_locked(&self) -> Result<bool, APIError> {
        match api::get_data_buffer_state::exec(self.transport()) {
            Err(APIError::SecurityStatusNotSatisfied) => Ok(true),
            Ok(_) => Ok(false),
            Err(e) => Err(e),
        }
    }

    // convenience function for first getting the data buffer state and then
    // downloading as many blocks as needed from the device
    // it returns a vector with the size reported by the device
    fn read_data_bufer(&self) -> Result<Vec<u8>, APIError> {
        // get buffer state
        let dbs = api::get_data_buffer_state::exec(self.transport())?;

        // is buffer state okay? (read allowed, contains addresses, valid flag set)
        if dbs.data_type as u8 != constants::DataTypeEnum::GeneratedAddress as u8
            && dbs.data_type as u8 != constants::DataTypeEnum::GeneratedPublicKeys as u8
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
            api::write_data_block::exec(self.transport(), block, block_data)?;
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
    pub fn set_account(&self, coin_type: u32, bip32_account: u32) -> Result<(), APIError> {
        let app_config = crate::api::get_app_config::exec(self.transport())?;

        api::set_account::exec(coin_type, app_config, self.transport(), bip32_account)?;
        Ok(())
    }

    pub fn get_addresses(
        &self,
        show: bool,
        bip32: LedgerBIP32Index,
        count: usize,
    ) -> Result<Vec<[u8; constants::ADDRESS_SIZE_BYTES]>, api::errors::APIError> {
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

        Ok(addresses)
    }

    pub fn get_public_keys(
        &self,
        show: bool,
        bip32: LedgerBIP32Index,
        count: usize,
    ) -> Result<Vec<[u8; constants::PUBLIC_KEY_SIZE_BYTES]>, api::errors::APIError> {
        // generate public key api call exists >= 0.8.7
        if self.version < MINIMUM_APP_VERSION_GENERATE_PUBLIC_KEYS {
            return Err(APIError::AppTooOld);
        }

        // clear data buffer before public keys can be generated
        api::clear_data_buffer::exec(self.transport())?;

        let max_count = self.data_buffer_size / constants::PUBLIC_KEY_SIZE_BYTES;

        if count > max_count {
            return Err(api::errors::APIError::CommandInvalidData);
        }

        // generate one or more public key(s)
        api::generate_public_key::exec(self.transport(), show, bip32, count as u32)?;

        // read addresses from device
        let buffer = self.read_data_bufer()?;

        let mut public_keys: Vec<[u8; 32]> = Vec::new();
        for i in 0_usize..count {
            let addr = buffer
                [i * constants::PUBLIC_KEY_SIZE_BYTES..(i + 1) * constants::PUBLIC_KEY_SIZE_BYTES]
                .try_into()
                .unwrap(); // each 33 bytes one address
            public_keys.push(addr);
        }

        Ok(public_keys)
    }

    pub fn get_first_address(
        &self,
    ) -> Result<[u8; constants::ADDRESS_SIZE_BYTES], api::errors::APIError> {
        // clear data buffer before addresses can be generated
        api::clear_data_buffer::exec(self.transport())?;

        // generate one single address
        api::generate_address::exec(
            self.transport(),
            false, // non interactive
            LedgerBIP32Index {
                bip32_index: constants::HARDENED,
                bip32_change: constants::HARDENED,
            },
            1, // single address
        )?;

        // read addresses from device
        let buffer = self.read_data_bufer()?;

        // no need to copy address type byte!
        let addr = buffer[1..constants::ADDRESS_WITH_TYPE_SIZE_BYTES]
            .try_into()
            .unwrap();
        Ok(addr)
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

        // now validate essence
        api::prepare_signing::exec(self.transport(), has_remainder, remainder_index, remainder)?;

        // get buffer state
        let dbs = api::get_data_buffer_state::exec(self.transport())?;

        // if recognized length is not the buffer_len, something went wrong
        // during parsing
        if dbs.data_length != buffer_len as u16 {
            return Err(APIError::Unknown);
        }

        Ok(())
    }

    /// Prepare Blind Signing
    ///
    /// Uploads the essence hash and validates it
    pub fn prepare_blind_signing(
        &self,
        key_indices: Vec<LedgerBIP32Index>,
        essence_hash: Vec<u8>,
    ) -> Result<(), api::errors::APIError> {
        // clone buffer because we have to add the key indices after the essence
        let mut buffer: Vec<u8> = essence_hash.to_vec();
        let key_number: u16 = key_indices.len() as u16;
        key_number
            .pack(&mut buffer)
            .map_err(|_| APIError::Unknown)?;

        for key in key_indices.iter() {
            key.pack(&mut buffer).map_err(|_| APIError::Unknown)?;
        }
        let buffer_len = buffer.len();

        // write data to the device
        self.write_data_buffer(buffer)?;

        // now validate essence
        api::prepare_blind_signing::exec(self.transport())?;

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

    /// Sign
    ///
    /// The publicly usable function for signing an essence.
    pub fn sign(&self, num_inputs: u16) -> Result<Vec<u8>, api::errors::APIError> {
        let mut signatures: Vec<u8> = Vec::new();

        for signature_idx in 0..num_inputs as u8 {
            let mut signature = api::sign::exec(self.transport(), signature_idx)?;
            signatures.append(&mut signature.data);
        }

        Ok(signatures)
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
