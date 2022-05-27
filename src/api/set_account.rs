use crate::Transport;
use ledger_apdu::APDUCommand;

use crate::api::{constants, errors, helpers};

// avoid dependencies to bee in this low-level lib
//use bee_common_ext::packable::{Error as PackableError, Packable, Read, Write};
use crate::api::packable::{Error as PackableError, Packable, Read, Write};

#[derive(Debug)]
pub struct Request {
    pub bip32_account: u32,
}

impl Packable for Request {
    fn packed_len(&self) -> usize {
        0u32.packed_len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        self.bip32_account.pack(buf)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(Self {
            bip32_account: u32::unpack(buf)?,
        })
    }
}

pub fn exec(coin_type: u32, transport: &Transport, account: u32) -> Result<(), errors::APIError> {
    let req = Request {
        bip32_account: account,
    };

    let mut buf = Vec::new();
    let _ = req.pack(&mut buf);

    let app_mode = match coin_type {
        0x107a => 0x00, // iota + chrysalis
        0x107b => 0x03, // shimmer + stardust
        _ => return Err(errors::APIError::IncorrectP1P2),
    };

    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::SetAccount as u8,
        p1: app_mode,
        p2: 0,
        data: buf,
    };
    helpers::exec::<()>(transport, cmd)
}
