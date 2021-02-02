use futures::executor;

// avoid dependencies to bee in this low-level lib
//use bee_common_ext::packable::{Error as PackableError, Packable, Read, Write};
use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use ledger_apdu::APDUCommand;
use ledger_transport::Exchange;

use crate::api::{errors, packable};

impl Packable for () {
    fn packed_len(&self) -> usize {
        0
    }

    fn pack<W: Write>(&self, _buf: &mut W) -> Result<(), PackableError> {
        Ok(())
    }

    fn unpack<R: Read>(_buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(())
    }
}

pub fn exec<T: packable::Packable>(
    transport: &dyn Exchange,
    cmd: APDUCommand,
) -> Result<T, errors::APIError> {
    match executor::block_on(transport.exchange(&cmd)) {
        Ok(resp) => {
            if resp.retcode != 0x9000 {
                return Err(errors::APIError::get_error(resp.retcode));
            }
            let res = T::unpack(&mut &resp.data[..]).map_err(|_| errors::APIError::Unknown)?;
            Ok(res)
        }
        Err(e) => {
            log::error!("error: {}", e);
            Err(errors::APIError::TransportError)
        }
    }
}
