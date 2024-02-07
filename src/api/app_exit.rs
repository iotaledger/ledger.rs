use crate::ledger::ledger_apdu::APDUCommand;
use crate::Transport;

use crate::api::{constants, errors, helpers};

pub fn exec(transport: &Transport) -> Result<(), errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASSB0,
        ins: constants::APDUInstructionsBolos::AppExitB0 as u8,
        p1: 0,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<()>(transport, cmd)
}
