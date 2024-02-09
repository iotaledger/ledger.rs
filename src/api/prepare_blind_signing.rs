use crate::ledger::ledger_apdu::APDUCommand;
use crate::Transport;

use crate::api::{constants, errors, helpers};

pub fn exec(transport: &Transport, num_hashes: u8) -> Result<(), errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::PrepareBlindsigning as u8,
        p1: num_hashes,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<()>(transport, cmd)
}
