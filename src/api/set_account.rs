use crate::ledger::ledger_apdu::APDUCommand;
use crate::Transport;

use crate::api::get_app_config;
use crate::api::{
    constants,
    constants::{AppModes, Apps},
    errors, helpers,
};

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

pub fn exec(
    coin_type: u32,
    app_config: get_app_config::Response,
    transport: &Transport,
    account: u32,
) -> Result<(), errors::APIError> {
    let req = Request {
        bip32_account: account,
    };

    let mut buf = Vec::new();
    let _ = req.pack(&mut buf);

    let flags = get_app_config::AppConfigFlags::from(app_config.flags);

    if ![0x1, 0x107a, 0x107b].contains(&coin_type) {
        return Err(errors::APIError::IncorrectP1P2);
    }

    // IOTA App
    // 0x00: (107a) IOTA + Chrysalis (default, backwards compatible)
    // 0x80:    (1) IOTA + Chrysalis Testnet
    // 0x01: (107a) IOTA + Stardust
    // 0x81:    (1) IOTA + Stardust Testnet

    // Shimmer App
    // 0x02: (107a) Shimmer Claiming (from IOTA)
    // 0x82:    (1) Shimmer Claiming (from IOTA) (Testnet)
    // 0x03: (107b) Shimmer (default)
    // 0x83:    (1) Shimmer Testnet

    let app_mode = match flags.app {
        Apps::AppIOTA => match coin_type {
            // 0x107a => AppModes::ModeIOTAChrysalis,
            // 0x1 => AppModes::ModeIOTAChrysalisTestnet,
            // IOTA + stardust
            0x107a => AppModes::ModeIOTAStardust,
            0x1 => AppModes::ModeIOTAStardustTestnet,
            _ => return Err(errors::APIError::IncorrectP1P2),
        },
        Apps::AppShimmer => match coin_type {
            // shimmer claiming
            0x107a => AppModes::ModeShimmerClaiming,
            // shimmer
            0x107b => AppModes::ModeShimmer,
            // shimmer claiming / shimmer testnet
            // use account to differenciate if claiming or not
            0x1 => {
                if account & 0x40000000 != 0 {
                    AppModes::ModeShimmerClaimingTestnet
                } else {
                    AppModes::ModeShimmerTestnet
                }
            }
            _ => return Err(errors::APIError::IncorrectP1P2),
        },
    };

    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::SetAccount as u8,
        p1: app_mode as u8,
        p2: 0,
        data: buf,
    };
    helpers::exec::<()>(transport, cmd)
}
