use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use crate::Transport;
use crate::ledger::ledger_apdu::APDUCommand;

use crate::api::constants::DataTypeEnum;
use crate::api::{constants, errors, helpers};

#[derive(Debug)]
pub struct Response {
    pub data_length: u16,
    pub data_type: DataTypeEnum,
    pub data_block_size: u8,
    pub data_block_count: u8,
}

impl Packable for Response {
    fn packed_len(&self) -> usize {
        0u16.packed_len() + 0u8.packed_len() + 0u8.packed_len() + 0u8.packed_len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        let data_type_u8: u8 = self.data_type as u8;
        data_type_u8.pack(buf)?;

        self.data_block_size.pack(buf)?;
        self.data_block_count.pack(buf)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        let data_length = u16::unpack(buf)?;
        let data_type_u8 = u8::unpack(buf)?;
        let data_block_size = u8::unpack(buf)?;
        let data_block_count = u8::unpack(buf)?;

        let data_type: DataTypeEnum = DataTypeEnum::get_type(data_type_u8);

        Ok(Self {
            data_length,
            data_type,
            data_block_size,
            data_block_count,
        })
    }
}

pub fn exec(transport: &Transport) -> Result<Response, errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::GetDataBufferState as u8,
        p1: 0,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<Response>(transport, cmd)
}
