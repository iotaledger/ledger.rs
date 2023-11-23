use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use crate::ledger::ledger_apdu::APDUCommand;
use crate::Transport;

use crate::api::{constants, errors, helpers};

use std::fs::File;

#[derive(Debug)]
pub struct Response {
    pub data: Vec<u8>,
}

impl Packable for Response {
    fn packed_len(&self) -> usize {
        128
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        buf.write_all(&self.data)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        let mut data = [0u8; 128];
        buf.read_exact(&mut data)?;

        Ok(Self {
            data: data.to_vec(),
        })
    }
}

impl Response {}

pub fn exec(transport: &Transport, block_number: u8) -> Result<Response, errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::DumpMemory as u8,
        p1: block_number,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<Response>(transport, cmd)
}

pub fn read(transport: &Transport, size: usize) -> Result<Vec<u8>, errors::APIError> {
    let mut mem: Vec<u8> = Vec::new();
    for i in 0..(size / 128) as u8 {
        let mut block = exec(transport, i)?;
        mem.append(&mut block.data);
    }
    Ok(mem)
}

pub fn memory_dump(transport: &Transport, filename: String) -> Result<(), errors::APIError> {
    let res = crate::api::get_app_config::exec(transport)?;

    let sram_size = match res.device {
        0 => 4 * 1024 + 512, // firmware 2.0.0
        1 => 30 * 1024,
        _ => {
            return Err(errors::APIError::Unknown);
        }
    };

    let memory = read(transport, sram_size)?;
    let mut file = File::create(filename).map_err(|_| errors::APIError::Unknown)?;
    let _ = file.write_all(&memory);
    Ok(())
}
