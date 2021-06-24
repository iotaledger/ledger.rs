use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use ledger_apdu::APDUCommand;
use ledger_transport::Exchange;

use crate::api::{constants, errors, helpers};

#[derive(Debug)]
pub struct Request {
    pub bip32_index: u32,
    pub bip32_change: u32,
    pub count: u32,
}

impl Packable for Request {
    fn packed_len(&self) -> usize {
        0u32.packed_len() + 0u32.packed_len() + 0u32.packed_len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        self.bip32_index.pack(buf)?;
        self.bip32_change.pack(buf)?;
        self.count.pack(buf)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(Self {
            bip32_index: u32::unpack(buf)?,
            bip32_change: u32::unpack(buf)?,
            count: u32::unpack(buf)?,
        })
    }
}

pub fn exec(
    transport: &dyn Exchange,
    show: bool,
    bip32: crate::LedgerBIP32Index,
    count: u32,
) -> Result<(), errors::APIError> {
    let req = Request {
        bip32_index: bip32.bip32_index,
        bip32_change: bip32.bip32_change,
        count,
    };

    let mut buf = Vec::new();
    let _ = req.pack(&mut buf);

    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::GenerateAddresses as u8,
        p1: if show { 1 } else { 0 },
        p2: 0u8,
        data: buf,
    };
    helpers::exec::<()>(transport, cmd)
}
