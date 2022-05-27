use std::{thread, time};

use crate::Transport;
use ledger_apdu::APDUCommand;

use crate::api::{constants, errors, helpers};

pub fn exec(transport: &crate::Transport, flow: constants::Flows) -> Result<(), errors::APIError> {
    let cmd = APDUCommand {
        cla: constants::APDUCLASS,
        ins: constants::APDUInstructions::ShowFlow as u8,
        p1: flow as u8,
        p2: 0,
        data: Vec::new(),
    };
    helpers::exec::<()>(transport, cmd)
}

pub fn show_main_menu(transport: &crate::Transport) -> Result<(), errors::APIError> {
    exec(transport, constants::Flows::FlowMainMenu)
}

#[allow(dead_code)]
pub fn show_generating_addresses(transport: &crate::Transport) -> Result<(), errors::APIError> {
    exec(transport, constants::Flows::FlowGeneratingAddresses)
}

#[allow(dead_code)]
pub fn show_signed_successfully(transport: &crate::Transport) -> Result<(), errors::APIError> {
    exec(transport, constants::Flows::FlowSignedSuccessfully)
}

pub fn show_signing(transport: &crate::Transport) -> Result<(), errors::APIError> {
    exec(transport, constants::Flows::FlowSigning)
}

pub fn show_for_ms(
    transport: &Transport,
    flow: constants::Flows,
    millis: u64,
) -> Result<(), errors::APIError> {
    exec(transport, flow)?;
    thread::sleep(time::Duration::from_millis(millis));

    show_main_menu(transport)?;
    Ok(())
}
