pub const HARDENED: u32 = 0x80000000;

pub const DATA_BLOCK_SIZE: usize = 251;
pub const APDUCLASS: u8 = 0x7b;

// everything is ED25519 ... no need for type
pub const ADDRESS_WITH_TYPE_SIZE_BYTES: usize = 33;
pub const ADDRESS_SIZE_BYTES: usize = 32;

pub enum APDUInstructions {
    None = 0x00,

    GetAppConfig = 0x10,
    SetAccount = 0x11,

    // data buffer instructions
    GetDataBufferState = 0x80,
    WriteDataBlock = 0x81,
    ReadDataBlock = 0x82,
    ClearDataBuffer = 0x83,

    ShowFlow = 0x90,
    PrepareBlindsigning = 0x91,

    // iota specific crypto instructions
    PrepareSigning = 0xa0,
    GenerateAddresses = 0xa1,
    Sign = 0xa2,
    UserConfirm = 0xa3,
    SignSingle = 0xa4,

    // commands for debug mode
    DumpMemory = 0x66,
    SetNonInteractiveMode = 0x67,

    Reset = 0xff,
}

pub(crate) const APDUCLASSB0: u8 = 0xb0;
pub(crate) const APDUCLASSE0: u8 = 0xe0;

pub(crate) enum APDUInstructionsBolos {
    GetAppVersionB0 = 0x01,
    AppExitB0 = 0xa7,

    OpenAppE0 = 0xd8,
}

pub enum Flows {
    FlowMainMenu = 0,
    FlowGeneratingAddresses = 1,
    FlowGenericError = 2,
    FlowRejected = 3,
    FlowSignedSuccessfully = 4,
    FlowSigning = 5,
}

#[derive(Debug, Copy, Clone)]
pub enum DataTypeEnum {
    Empty = 0,
    GeneratedAddress = 1,
    ValidatedEssence = 2,
    UserConfirmedEssence = 3,
    Signatures = 4,
    Locked = 5,

    Unknown = 255,
}

impl DataTypeEnum {
    pub fn get_type(i: u8) -> Self {
        match i {
            0 => Self::Empty,
            1 => Self::GeneratedAddress,
            2 => Self::ValidatedEssence,
            3 => Self::UserConfirmedEssence,
            4 => Self::Signatures,
            5 => Self::Locked,
            _ => Self::Unknown,
        }
    }
}
