pub mod transport_tcp;
pub mod errors;

use hidapi::HidApi;
use once_cell::sync::Lazy;

use ledger_transport_hid::TransportNativeHID;
use crate::transport::transport_tcp::{TransportTCP, Callback};
use crate::APIError;

fn hidapi() -> &'static HidApi {
    static HIDAPI: Lazy<HidApi> = Lazy::new(|| HidApi::new().expect("unable to get HIDAPI"));

    &HIDAPI
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
        TransportTypes::NativeHID => Transport::NativeHID(
            TransportNativeHID::new(hidapi()).map_err(|_| APIError::TransportError)?,
        ),
    };
    Ok(transport)
}