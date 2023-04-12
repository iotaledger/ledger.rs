use clap::{App, Arg};
use iota_ledger_nano::api::errors::APIError;
use iota_ledger_nano::transport::TransportTypes;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::result::Result;
use std::{thread, time::Duration};

/// The Ledger device status.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LedgerApp {
    /// Opened app name.
    pub(crate) name: String,
    /// Opened app version.
    pub(crate) version: String,
}

impl LedgerApp {
    /// Opened app name.
    pub fn name(&self) -> &String {
        &self.name
    }
    /// Opened app version.
    pub fn version(&self) -> &String {
        &self.version
    }
}

/// The Ledger device status.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LedgerNanoStatus {
    /// Ledger is available and ready to be used.
    pub(crate) connected: bool,
    /// Ledger is connected and locked.
    pub(crate) locked: bool,
    /// Ledger blind signing enabled
    pub(crate) blind_signing_enabled: bool,
    /// Ledger opened app.
    pub(crate) app: Option<LedgerApp>,
    /// Ledger device
    pub(crate) device: Option<LedgerDeviceType>,
    /// Buffer size on device
    #[serde(rename = "bufferSize")]
    pub(crate) buffer_size: Option<usize>,
}

/// Ledger Device Type
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LedgerDeviceType {
    /// Device Type Nano S
    #[serde(alias = "ledgerNanoS")]
    LedgerNanoS,
    /// Device Type Nano X
    #[serde(alias = "ledgerNanoX")]
    LedgerNanoX,
    /// Device Type Nano S Plus
    #[serde(alias = "ledgerNanoSPlus")]
    LedgerNanoSPlus,
}

impl TryFrom<u8> for LedgerDeviceType {
    type Error = APIError;
    fn try_from(device: u8) -> Result<Self, Self::Error> {
        match device {
            0 => Ok(Self::LedgerNanoS),
            1 => Ok(Self::LedgerNanoX),
            2 => Ok(Self::LedgerNanoSPlus),
            _ => Err(APIError::TransportError),
        }
    }
}

/// Get Ledger hardware status.
pub fn get_ledger_nano_status(is_simulator: bool) -> LedgerNanoStatus {
    log::debug!("get_ledger_nano_status");
    // lock the mutex
    let transport_type = if is_simulator {
        TransportTypes::TCP
    } else {
        TransportTypes::NativeHID
    };

    log::debug!("get_opened_app");
    let app = match iota_ledger_nano::get_opened_app(&transport_type) {
        Ok((name, version)) => Some(LedgerApp { name, version }),
        _ => None,
    };

    log::debug!("get_app_config");
    // if IOTA or Shimmer app is opened, the call will always succeed, returning information like
    // device, debug-flag, version number, lock-state but here we only are interested in a
    // successful call and the locked-flag
    let (connected_, locked, blind_signing_enabled, device) =
        match iota_ledger_nano::get_app_config(&transport_type) {
            Ok(config) => (
                true,
                // locked flag
                config.flags & (1 << 0) != 0,
                // blind signing enabled flag
                config.flags & (1 << 1) != 0,
                LedgerDeviceType::try_from(config.device).ok(),
            ),
            Err(_) => (false, false, false, None),
        };

    log::debug!("get_buffer_size");
    // get buffer size of connected device
    let buffer_size = match iota_ledger_nano::get_buffer_size(&transport_type) {
        Ok(size) => Some(size),
        Err(_) => None,
    };

    // We get the app info also if not the iota app is open, but another one
    // connected_ is in this case false, even tough the ledger is connected, that's why we always return true if we
    // got the app
    let connected = if app.is_some() { true } else { connected_ };
    LedgerNanoStatus {
        connected,
        locked,
        blind_signing_enabled,
        app,
        device,
        buffer_size,
    }
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

    loop {
        let status = get_ledger_nano_status(is_simulator);
        println!("{:?}", status);
        thread::sleep(Duration::from_millis(1000));
    }
}
