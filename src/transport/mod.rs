pub mod errors;

use crate::ledger::ledger_transport::{APDUAnswer, APDUCommand};
use crate::ledger::ledger_transport_hid::TransportNativeHID;
use crate::ledger::ledger_transport_tcp::{Callback, TransportTCP};
use crate::APIError;

use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

use std::time::{Duration, Instant};

use log::debug;

lazy_static! {
    static ref TRANSPORT_MUTEX: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
}

use std::sync::MutexGuard;

#[derive(Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum TransportTypes {
    TCP,
    NativeHID,
}

pub struct Transport {
    pub transport: LedgerTransport,
    _transport_mutex: MutexGuard<'static, i32>,
}

impl Drop for Transport {
    fn drop(&mut self) {
        debug!("transport_mutex released");
    }
}

#[allow(clippy::upper_case_acronyms)]
pub enum LedgerTransport {
    TCP(TransportTCP),
    NativeHID(TransportNativeHID),
}

impl LedgerTransport {
    pub(crate) fn exchange(
        &self,
        apdu_command: &APDUCommand<Vec<u8>>,
    ) -> Result<APDUAnswer<Vec<u8>>, APIError> {
        match self {
            LedgerTransport::TCP(t) => t
                .exchange(apdu_command)
                .map_err(|_| APIError::TransportError),
            LedgerTransport::NativeHID(h) => h
                .exchange(apdu_command)
                .map_err(|_| APIError::TransportError),
        }
    }
}

fn try_get_lock(timeout: Duration) -> Result<MutexGuard<'static, i32>, APIError> {
    let start_time = Instant::now();
    while start_time.elapsed() < timeout {
        match TRANSPORT_MUTEX.try_lock() {
            Ok(guard) => {
                return Ok(guard);
            }
            Err(_) => {
                debug!("trying to acquire transport_mutex lock...");
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    Err(APIError::Timeout)
}

// only create transport without IOTA specific calls
pub fn create_transport(
    transport_type: &TransportTypes,
    callback: Option<Callback>,
) -> Result<Transport, APIError> {
    debug!("transport_mutex try lock");
    let transport_mutex = try_get_lock(Duration::from_secs(30))?;
    debug!("transport_mutex locked");
    let transport = match transport_type {
        TransportTypes::TCP => Transport {
            _transport_mutex: transport_mutex,
            transport: LedgerTransport::TCP(TransportTCP::new("127.0.0.1", 9999, callback)),
        },
        TransportTypes::NativeHID => {
            let api = hidapi::HidApi::new().map_err(|_| APIError::TransportError)?;
            Transport {
                _transport_mutex: transport_mutex,
                transport: LedgerTransport::NativeHID(
                    TransportNativeHID::new(&api).map_err(|_| APIError::TransportError)?,
                ),
            }
        }
    };
    Ok(transport)
}
