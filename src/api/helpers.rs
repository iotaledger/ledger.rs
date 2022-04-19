use futures::executor;

use ledger_transport::APDUCommand;

use crate::api::{errors, packable};

pub fn exec<T: packable::Packable>(
    transport: &crate::Transport,
    cmd: APDUCommand<Vec<u8>>,
) -> Result<T, errors::APIError> {
    match executor::block_on(transport.exchange(&cmd)) {
        Ok(resp) => {
            if resp.retcode() != 0x9000 {
                return Err(errors::APIError::get_error(resp.retcode()));
            }
            let res = T::unpack(&mut &resp.data()[..]).map_err(|_| errors::APIError::Unknown)?;
            Ok(res)
        }
        Err(e) => {
            log::error!("error: {}", e);
            Err(errors::APIError::TransportError)
        }
    }
}
