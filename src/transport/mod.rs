pub mod errors;
pub mod transport_tcp;

use crate::transport::transport_tcp::{Callback, TransportTCP};
use crate::APIError;
use ledger_transport_hid::TransportNativeHID;

use lazy_static::lazy_static;
use std::cell::RefCell;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

struct HidApiWrapper {
    _api: RefCell<Weak<Mutex<hidapi::HidApi>>>,
}

lazy_static! {
    static ref HIDAPIWRAPPER: Arc<Mutex<HidApiWrapper>> =
        Arc::new(Mutex::new(HidApiWrapper::new()));
    static ref TRANSPORT_MUTEX: Mutex<i32> = Mutex::new(0);
}

impl HidApiWrapper {
    fn new() -> Self {
        HidApiWrapper {
            _api: RefCell::new(Weak::new()),
        }
    }

    fn get(&self) -> Result<Arc<Mutex<hidapi::HidApi>>, APIError> {
        let tmp = self._api.borrow().upgrade();

        if let Some(api_mutex) = tmp {
            return Ok(api_mutex);
        }

        let hidapi = hidapi::HidApi::new().map_err(|_| APIError::TransportError)?;
        let tmp = Arc::new(Mutex::new(hidapi));
        self._api.replace(Arc::downgrade(&tmp));
        Ok(tmp)
    }
}

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

#[allow(clippy::upper_case_acronyms)]
pub enum LedgerTransport {
    TCP(TransportTCP),
    NativeHID(TransportNativeHID),
}

impl LedgerTransport {
    pub(crate) async fn exchange(
        &self,
        apdu_command: &ledger_transport::APDUCommand<Vec<u8>>,
    ) -> Result<ledger_transport::APDUAnswer<Vec<u8>>, APIError> {
        match self {
            LedgerTransport::TCP(t) => t
                .exchange(apdu_command)
                .await
                .map_err(|_| APIError::TransportError),
            LedgerTransport::NativeHID(h) => h
                .exchange(apdu_command)
                .map_err(|_| APIError::TransportError),
        }
    }
}

// only create transport without IOTA specific calls
pub fn create_transport(
    transport_type: &TransportTypes,
    callback: Option<Callback>,
) -> Result<Transport, APIError> {
    let transport_mutex = TRANSPORT_MUTEX.lock().unwrap();
    let transport = match transport_type {
        TransportTypes::TCP => Transport {
            _transport_mutex: transport_mutex,
            transport: LedgerTransport::TCP(TransportTCP::new("127.0.0.1", 9999, callback)),
        },
        TransportTypes::NativeHID => {
            let apiwrapper = HIDAPIWRAPPER.lock().map_err(|_| APIError::TransportError)?;
            let api_mutex = apiwrapper.get()?;
            let api = api_mutex.lock().map_err(|_| APIError::TransportError)?;

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
