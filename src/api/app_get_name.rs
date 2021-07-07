use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use ledger_apdu::APDUCommand;
use ledger_transport::Exchange;

use crate::api::{constants, errors, helpers};
/*
dashboard:
HID => b001000000
HID <= 0105|424f4c4f53|05|322e302e30|9000
            B O L O S      2 . 0 . 0

"IOTA Legacy"
HID => b001000000
HID <= 010b|494f5441204c6567616379|05|302e352e38|0102|9000
             I O T A   L e g a c y     0 . 5 . 8

"IOTA"
HID => b001000000
HID <= 0104|494f5441|05|302e372e30|0102|9000
            I O T A      0 . 7 . 0
*/

#[derive(Debug)]
pub struct Response {
    pub format_id: u8,
    pub app: String,
    pub version: String,
    pub flags: u8,
}

impl Packable for Response {
    fn packed_len(&self) -> usize {
        self.format_id.packed_len()
            + self.app.packed_len()
            + self.version.packed_len()
            + if self.app == "BOLOS" {
                0
            } else {
                0u8.packed_len() + 0u8.packed_len()
            }
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        self.format_id.pack(buf)?;
        self.app.pack(buf)?;
        self.version.pack(buf)?;

        // two extra bytes if app is not BOLOS
        if self.app != "BOLOS" {
            1u8.pack(buf)?;
            self.flags.pack(buf)?;
        }

        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        // format always 0x01 but don't insist on it
        let format_id = u8::unpack(buf)?;

        let app = String::unpack(buf)?;
        let version = String::unpack(buf)?;

        // dashboard app doesn't give flags
        let flags = if app == "BOLOS" {
            0x00
        } else {
            // consume all extra bytes (nano x <-> nano s compatibility!)
            loop {
                let u = u8::unpack(buf);
                if u.is_ok() {
                    continue;
                }
                break;
            }
            0
        };
        Ok(Self {
            format_id,
            app,
            version,
            flags,
        })
    }
}

impl Response {
    // NOP
}

pub fn exec(transport: &dyn Exchange) -> Result<Response, errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASSB0,
        ins: constants::APDUInstructionsBolos::GetAppVersionB0 as u8,
        p1: 0,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<Response>(transport, cmd)
}
