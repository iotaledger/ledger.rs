use ledger_transport::APDUCommand;

use crate::api::{constants, errors, helpers};

pub fn exec(
    transport: &crate::Transport,
    block_number: u8,
    data: Vec<u8>,
) -> Result<(), errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::WriteDataBlock as u8,
        p1: block_number,
        p2: 0,
        data,
    };
    helpers::exec::<()>(transport, cmd)
}
