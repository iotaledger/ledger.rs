use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use ledger_transport::APDUCommand;

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
        0
    }

    fn pack<W: Write>(&self, _buf: &mut W) -> Result<(), PackableError> {
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        // format always 0x01 but don't insist on it
        let format_id = u8::unpack(buf)?;

        let mut app = String::unpack(buf)?;

        // nanox sdk bug
        // https://github.com/LedgerHQ/nanox-secure-sdk/issues/6
        // reported as OLOS\0
        if app.chars().count() == 5 && app.as_bytes() == [0x4f, 0x4c, 0x4f, 0x53, 0x00] {
            // unquirk dashboard name
            app = String::from("BOLOS");
        }

        // version is corrupted on nanox sdk - but we don't need it anyway
        // the reported length is ok
        let version = String::unpack(buf)?;

        // consume all extra bytes (nano x <-> nano s compatibility!)
        while u8::unpack(buf).is_ok() {
            // NOP
        }

        Ok(Self {
            format_id,
            app,
            version,
            flags: 0,
        })
    }
}

impl Response {
    // NOP
}

pub fn exec(transport: &crate::Transport) -> Result<Response, errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASSB0,
        ins: constants::APDUInstructionsBolos::GetAppVersionB0 as u8,
        p1: 0,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<Response>(transport, cmd)
}
