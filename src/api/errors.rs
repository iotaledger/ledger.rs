use thiserror::Error;

#[derive(Error, Debug)]
pub enum APIError {
    #[error("No error")]
    Ok,

    #[error("Incorrect length")]
    IncorrectLength,

    #[error("Invalid data")]
    CommandInvalidData,

    #[error("Incorrect P1 or P2 parameter")]
    IncorrectP1P2,

    #[error("Incorrect length P3")]
    IncorrectLengthP3,

    #[error("Instruction not supported")]
    InstructionNotSupported,

    #[error("Class not supported")]
    ClassNotSupported,

    #[error("Command not allowed")]
    CommandNotAllowed,

    #[error("Security status not satisfied")]
    SecurityStatusNotSatisfied, // dongle locked

    #[error("Conditions of use not satisfied")]
    ConditionsOfUseNotSatisfied, // denied by user

    #[error("Command timeout")]
    CommandTimeout,

    #[error("Transport error")]
    TransportError,

    #[error("Essence too large")]
    EssenceTooLarge,

    #[error("unknown")]
    Unknown,
}

impl APIError {
    pub fn get_error(rc: u16) -> APIError {
        match rc {
            0x9000 => APIError::Ok,
            0x6700 => APIError::IncorrectLength,
            0x6a80 => APIError::CommandInvalidData,
            0x6b00 => APIError::IncorrectP1P2,
            0x6c00 => APIError::IncorrectLengthP3,
            0x6d00 => APIError::InstructionNotSupported,
            0x6e00 => APIError::ClassNotSupported,
            0x6900 => APIError::CommandNotAllowed,
            0x6982 => APIError::SecurityStatusNotSatisfied,
            0x6985 => APIError::ConditionsOfUseNotSatisfied,
            0x6401 => APIError::CommandTimeout,
            _ => APIError::Unknown,
        }
    }
}
