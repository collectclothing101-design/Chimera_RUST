// chimera-firmware/src/samsung_fw.rs
// Samsung firmware info structures

pub struct SamsungFirmwareInfo {
    pub model: String,
    pub region: String,
    pub version: String,
    pub pda: String,
    pub csc_pda: String,
    pub phone_pda: String,
}
