use ledger_transport::APDUCommand;

use crate::api::{constants, errors, helpers};

pub fn exec(transport: &crate::Transport) -> Result<(), errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::UserConfirm as u8,
        p1: 0,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<()>(transport, cmd)
}
