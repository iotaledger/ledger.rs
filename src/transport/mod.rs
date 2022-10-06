pub mod errors;
pub mod transport_tcp;

use crate::transport::transport_tcp::{Callback, TransportTCP};
use crate::APIError;
use ledger_transport_hid::TransportNativeHID;

use lazy_static::lazy_static;
use std::cell::RefCell;
use std::sync::{Arc, Mutex, Weak};

struct HidApiWrapper {
    _api: RefCell<Weak<Mutex<hidapi::HidApi>>>,
}

lazy_static! {
    static ref HIDAPIWRAPPER: Arc<Mutex<HidApiWrapper>> =
        Arc::new(Mutex::new(HidApiWrapper::new()));
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

#[allow(clippy::upper_case_acronyms)]
pub enum Transport {
    TCP(TransportTCP),
    NativeHID(TransportNativeHID),
}

impl Transport {
    pub(crate) async fn exchange(
        &self,
        apdu_command: &ledger_transport::APDUCommand<Vec<u8>>,
    ) -> Result<ledger_transport::APDUAnswer<Vec<u8>>, APIError> {
        match self {
            Transport::TCP(t) => t
                .exchange(apdu_command)
                .await
                .map_err(|_| APIError::TransportError),
            Transport::NativeHID(h) => h
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
    let transport = match transport_type {
        TransportTypes::TCP => Transport::TCP(TransportTCP::new("127.0.0.1", 9999, callback)),
        TransportTypes::NativeHID => {
            let apiwrapper = HIDAPIWRAPPER.lock().map_err(|_| APIError::TransportError)?;
            let api_mutex = apiwrapper.get()?;
            let api = api_mutex.lock().map_err(|_| APIError::TransportError)?;

            Transport::NativeHID(
                TransportNativeHID::new(&api).map_err(|_| APIError::TransportError)?,
            )
        }
    };
    Ok(transport)
}
