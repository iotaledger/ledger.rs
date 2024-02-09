pub const HARDENED: u32 = 0x80000000;

pub const DATA_BLOCK_SIZE: usize = 251;
pub const APDUCLASS: u8 = 0x7b;

// everything is ED25519 ... no need for type
pub const ADDRESS_WITH_TYPE_SIZE_BYTES: usize = 33;
pub const ADDRESS_SIZE_BYTES: usize = 32;
pub const PUBLIC_KEY_SIZE_BYTES: usize = 32;

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
    GeneratePublicKeys = 0xa5,

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

#[derive(Debug, PartialEq)]
pub enum Apps {
    AppIOTA = 0,
    AppShimmer = 1,
    Unknown,
}

#[derive(Copy, Clone)]
pub enum CoinType {
    IOTA = 0x107a,
    Shimmer = 0x107b,
    Testnet = 0x1,
}

#[derive(Copy, Clone)]
pub enum AppModes {
    ModeIOTAStardust = 0x01,
    ModeIOTAStardustTestnet = 0x81,
    ModeShimmerClaiming = 0x02,
    ModeShimmerClaimingTestnet = 0x82,
    ModeShimmer = 0x03,
    ModeShimmerTestnet = 0x83,
    ModeIOTANova = 0x04,
    ModeIOTANovaTestnet = 0x84,
    ModeShimmerNova = 0x05,
    ModeShimmerNovaTestnet = 0x85,
}

impl From<AppModes> for CoinType {
    fn from(app_mode: AppModes) -> Self {
        match app_mode {
            AppModes::ModeShimmerClaiming | AppModes::ModeIOTAStardust | AppModes::ModeIOTANova => {
                CoinType::IOTA
            }
            AppModes::ModeShimmer | AppModes::ModeShimmerNova => CoinType::Shimmer,
            AppModes::ModeShimmerClaimingTestnet
            | AppModes::ModeIOTAStardustTestnet
            | AppModes::ModeShimmerTestnet
            | AppModes::ModeIOTANovaTestnet
            | AppModes::ModeShimmerNovaTestnet => CoinType::Testnet,
        }
    }
}

impl From<AppModes> for Apps {
    fn from(app_mode: AppModes) -> Self {
        match app_mode {
            AppModes::ModeShimmerClaiming
            | AppModes::ModeIOTAStardust
            | AppModes::ModeIOTAStardustTestnet
            | AppModes::ModeIOTANova
            | AppModes::ModeIOTANovaTestnet => Apps::AppIOTA,
            AppModes::ModeShimmerClaimingTestnet
            | AppModes::ModeShimmer
            | AppModes::ModeShimmerTestnet
            | AppModes::ModeShimmerNova
            | AppModes::ModeShimmerNovaTestnet => Apps::AppShimmer,
        }
    }
}

impl From<u8> for Apps {
    fn from(app: u8) -> Self {
        match app {
            0 => Apps::AppIOTA,
            1 => Apps::AppShimmer,
            _ => Apps::Unknown,
        }
    }
}
#[derive(Debug, Copy, Clone)]
pub enum DataTypeEnum {
    Empty = 0,
    GeneratedAddress = 1,
    ValidatedEssence = 2,
    UserConfirmedEssence = 3,
    Signatures = 4,
    Locked = 5,
    GeneratedPublicKeys = 6,

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
            6 => Self::GeneratedPublicKeys,
            _ => Self::Unknown,
        }
    }
}

pub struct AppConfigFlags {
    pub locked: bool,
    pub blindsigning_enabled: bool,
    pub app: Apps,
}

impl From<u8> for AppConfigFlags {
    fn from(flags: u8) -> Self {
        Self {
            locked: flags & 0x01 != 0,
            blindsigning_enabled: flags & 0x02 != 0,
            app: if flags & 0x04 != 0 {
                Apps::AppShimmer
            } else {
                Apps::AppIOTA
            },
        }
    }
}
