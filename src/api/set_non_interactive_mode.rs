use crate::Transport;
use ledger_apdu::APDUCommand;

use crate::api::{constants, errors, helpers};

pub fn exec(transport: &Transport, non_interactive_mode: bool) -> Result<(), errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::SetNonInteractiveMode as u8,
        p1: if non_interactive_mode { 1 } else { 0 },
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<()>(transport, cmd)
}
