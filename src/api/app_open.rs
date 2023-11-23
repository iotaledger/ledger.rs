use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use crate::Transport;
use crate::ledger::ledger_apdu::APDUCommand;

use crate::api::{constants, errors, helpers};
/*
  E0D8000007|494f5441|
              I O T A
*/

#[derive(Debug)]
pub struct Request {
    pub app: String,
}

impl Packable for Request {
    fn packed_len(&self) -> usize {
        self.app.packed_len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        self.app.pack(buf)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(Self {
            app: String::unpack(buf)?,
        })
    }
}

pub fn exec(transport: &Transport, app: String) -> Result<(), errors::APIError> {
    let req = Request { app };

    let mut buf = Vec::new();
    let _ = req.pack(&mut buf);

    // string serializer stores a length byte that is unwanted here because
    // the p3 parameter will be the length of the string and the data itself
    // must not contain the length
    buf.remove(0);

    let cmd = APDUCommand {
        cla: constants::APDUCLASSE0,
        ins: constants::APDUInstructionsBolos::OpenAppE0 as u8,
        p1: 0,
        p2: 0,
        data: buf,
    };
    helpers::exec::<()>(transport, cmd)
}
