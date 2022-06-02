use crate::api::packable::{Error as PackableError, Packable, Read, Write};

use crate::Transport;
use ledger_apdu::APDUCommand;

use crate::api::{constants, errors, helpers};

#[derive(Debug)]
pub struct Response {
    pub app_version_major: u8,
    pub app_version_minor: u8,
    pub app_version_patch: u8,
    pub flags: u8,
    pub device: u8,
    pub is_debug_app: u8,
}

pub struct AppConfigFlags {
    pub locked: bool,
    pub blindsigning_enabled: bool,
    pub app: constants::Apps,
}

impl From<u8> for AppConfigFlags {
    fn from(flags: u8) -> Self {
        Self {
            locked: flags & 0x01 != 0,
            blindsigning_enabled: flags & 0x02 != 0,
            app: if flags & 0x04 != 0 {
                constants::Apps::AppShimmer
            } else {
                constants::Apps::AppIOTA
            },
        }
    }
}

impl Packable for Response {
    fn packed_len(&self) -> usize {
        0u8.packed_len()
            + 0u8.packed_len()
            + 0u8.packed_len()
            + 0u8.packed_len()
            + 0u8.packed_len()
            + 0u8.packed_len()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        self.app_version_major.pack(buf)?;
        self.app_version_minor.pack(buf)?;
        self.app_version_patch.pack(buf)?;
        self.flags.pack(buf)?;
        self.device.pack(buf)?;
        self.is_debug_app.pack(buf)?;
        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(Self {
            app_version_major: u8::unpack(buf)?,
            app_version_minor: u8::unpack(buf)?,
            app_version_patch: u8::unpack(buf)?,
            flags: u8::unpack(buf)?,
            device: u8::unpack(buf)?,
            is_debug_app: u8::unpack(buf)?,
        })
    }
}

impl Response {
    // NOP
}

pub fn exec(transport: &Transport) -> Result<Response, errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::GetAppConfig as u8,
        p1: 0,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<Response>(transport, cmd)
}
