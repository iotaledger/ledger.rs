use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use crate::ledger::ledger_apdu::APDUCommand;
use crate::Transport;

use crate::api::{constants, errors, helpers};

#[derive(Debug)]
pub struct Request {
    pub remainder_index: u16,
    pub remainder_bip32_index: u32,
    pub remainder_bip32_change: u32,
}

impl Packable for Request {
    fn packed_len(&self) -> usize {
        0u16.packed_len() + 0u32.packed_len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        self.remainder_index.pack(buf)?;
        self.remainder_bip32_index.pack(buf)?;
        self.remainder_bip32_change.pack(buf)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(Self {
            remainder_index: u16::unpack(buf)?,
            remainder_bip32_index: u32::unpack(buf)?,
            remainder_bip32_change: u32::unpack(buf)?,
        })
    }
}

pub fn exec(
    transport: &Transport,
    has_remainder: bool,
    remainder_index: u16,
    remainder: crate::LedgerBIP32Index,
) -> Result<(), errors::APIError> {
    let req = Request {
        remainder_index,
        remainder_bip32_index: remainder.bip32_index,
        remainder_bip32_change: remainder.bip32_change,
    };

    let mut buf = Vec::new();
    let _ = req.pack(&mut buf);

    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::PrepareSigning as u8,
        p1: 1, // compatibility
        p2: if has_remainder { 1 } else { 0 },
        data: buf,
    };
    helpers::exec::<()>(transport, cmd)
}
