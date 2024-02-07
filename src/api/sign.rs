use crate::ledger::ledger_apdu::APDUCommand;
use crate::Transport;

use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use crate::api::{constants, errors, helpers};

const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
const ED25519_SIGNATURE_LENGTH: usize = 64;

const SIGNATURE_UNLOCK_BLOCK_LENGTH: usize =
    1 + 1 + ED25519_PUBLIC_KEY_LENGTH + ED25519_SIGNATURE_LENGTH;
const REFERENCE_UNLOCK_BLOCK_LENGTH: usize = 1 + 2;

#[derive(Debug)]
pub struct ResponseVec {
    pub data: Vec<u8>,
}

impl Packable for ResponseVec {
    fn packed_len(&self) -> usize {
        self.data.len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        buf.write_all(&self.data)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        let mut data: Vec<u8> = Vec::new();

        let signature_type = u8::unpack(buf)?;
        let mut expected_length = match signature_type {
            0 => SIGNATURE_UNLOCK_BLOCK_LENGTH,
            1 => REFERENCE_UNLOCK_BLOCK_LENGTH,
            _ => {
                return Err(PackableError::InvalidVariant);
            }
        };

        expected_length -= 1;
        data.push(signature_type);

        let mut bytes_read = 0;
        loop {
            let byte = u8::unpack(buf)?;
            data.push(byte);

            bytes_read += 1;
            if bytes_read == expected_length {
                break;
            }
        }
        Ok(Self { data })
    }
}

pub fn exec(transport: &Transport, signature_index: u8) -> Result<ResponseVec, errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::SignSingle as u8,
        p1: signature_index,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<ResponseVec>(transport, cmd)
}
