use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use ledger_apdu::APDUCommand;
use ledger_transport::Exchange;

use crate::api::{constants, errors, helpers};

#[derive(Debug)]
pub struct Response {
    pub data: Vec<u8>,
}

impl Packable for Response {
    fn packed_len(&self) -> usize {
        constants::DATA_BLOCK_SIZE
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        buf.write_all(&self.data)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        let mut data = [0u8; constants::DATA_BLOCK_SIZE];
        buf.read_exact(&mut data)?;
        Ok(Self {
            data: data.to_vec(),
        })
    }
}

impl Response {}

pub fn exec(transport: &dyn Exchange, block_number: u8) -> Result<Response, errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::ReadDataBlock as u8,
        p1: block_number,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<Response>(transport, cmd)
}
