use crate::api::packable::{Error as PackableError, Packable, Read, Write};
use crate::{LedgerBIP32Index, LedgerBIP32IndexShort};

impl Packable for LedgerBIP32Index {
    fn packed_len(&self) -> usize {
        0u32.packed_len() + 0u32.packed_len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        self.bip32_index.pack(buf)?;
        self.bip32_change.pack(buf)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(Self {
            bip32_index: u32::unpack(buf)?,
            bip32_change: u32::unpack(buf)?,
        })
    }
}

impl Packable for LedgerBIP32IndexShort {
    fn packed_len(&self) -> usize {
        0u32.packed_len() + 0u8.packed_len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        self.bip32_index.pack(buf)?;
        self.bip32_change.pack(buf)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(Self {
            bip32_index: u32::unpack(buf)?,
            bip32_change: u8::unpack(buf)?,
        })
    }
}

impl From<&LedgerBIP32Index> for LedgerBIP32IndexShort {
    fn from(long: &LedgerBIP32Index) -> Self {
        Self {
            bip32_index: long.bip32_index,
            bip32_change: (long.bip32_change & 0xff) as u8,
        }
    }
}
