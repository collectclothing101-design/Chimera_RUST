//! Exynos USB Boot (**EUB**) mode — the low-level interface for Samsung
//! devices with Exynos SoCs. Equivalent to Qualcomm's EDL mode.
//!
//! Documented at <https://chimeratool.com/docs/samsung-exynos-devices-connect-the-device-in-eub-mode>
//! and supported chip families at <https://chimeratool.com/docs/all-supported-exynos-models>.
//!
//! ## Supported chip families
//!
//! Samsung Exynos 8825 / 9630 (alias 980) / 3830 (alias 850) /
//! 8535 (alias 1330) / 8835 (alias 1380) / 7884B / 7904 / 9610 / 9611 /
//! 7885 / 7904 / 7920 / 980 / 990.
//!
//! ## Procedures available from EUB mode
//!
//! - Restore / Store Backup
//! - Unlock / Relock Bootloader
//! - CSC Change
//! - Repair Serial
//! - Factory Reset
//! - Remove FMM (Find My Mobile)
//! - Remove FRP
//! - Knoxguard Remove
//! - Remove Lost Mode
//! - Remove Common Criteria mode
//! - Remove MDM
//! - Remove Warnings (bootloader-unlock warning at boot)
//! - Restore Security Backup
//! - Read Codes (all 6 in one step)
//! - Network Factory Reset
//! - Patch CERT
//! - Repair IMEI
//! - Repair Boot
//! - Set Knoxguard State
//! - Demo Remove (LDU flag)
//! - Change to EUB (transition from ODIN)
//! - Network Repair

use serde::{Serialize, Deserialize};
use chimera_core::error::Result;

/// Samsung Exynos chip family identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExynosChip {
    /// 8825 alias Exynos 1280 (Galaxy A33/M33/A53)
    Exynos8825_1280,
    /// 9630 alias Exynos 980 (Galaxy A51 5G/A71 5G)
    Exynos9630_980,
    /// 3830 alias Exynos 850 (Galaxy A21s/M02s)
    Exynos3830_850,
    /// 8535 alias Exynos 1330 (Galaxy A24/A15)
    Exynos8535_1330,
    /// 8835 alias Exynos 1380 (Galaxy A34/A54)
    Exynos8835_1380,
    /// 7884B (Galaxy A10s/A20)
    Exynos7884B,
    /// 7904 (Galaxy A30/A50)
    Exynos7904,
    /// 9610 (Galaxy A50/M30)
    Exynos9610,
    /// 9611 (Galaxy A51/M51)
    Exynos9611,
    /// 7885 (Galaxy A8/A7 2018)
    Exynos7885,
    /// 990 flagship
    Exynos990,
    /// 980 (Galaxy A71 5G)
    Exynos980,
    /// Other / unknown
    Unknown(u32),
}

impl ExynosChip {
    /// Display alias for the GUI (matches ChimeraTool naming).
    pub fn display(&self) -> &'static str {
        match self {
            ExynosChip::Exynos8825_1280 => "Exynos 1280 (8825)",
            ExynosChip::Exynos9630_980  => "Exynos 980 (9630)",
            ExynosChip::Exynos3830_850  => "Exynos 850 (3830)",
            ExynosChip::Exynos8535_1330 => "Exynos 1330 (8535)",
            ExynosChip::Exynos8835_1380 => "Exynos 1380 (8835)",
            ExynosChip::Exynos7884B     => "Exynos 7884B",
            ExynosChip::Exynos7904      => "Exynos 7904",
            ExynosChip::Exynos9610      => "Exynos 9610",
            ExynosChip::Exynos9611      => "Exynos 9611",
            ExynosChip::Exynos7885      => "Exynos 7885",
            ExynosChip::Exynos990       => "Exynos 990",
            ExynosChip::Exynos980       => "Exynos 980",
            ExynosChip::Unknown(_)      => "Exynos (unknown)",
        }
    }

    /// True when the chip is supported in EUB mode by the toolchain.
    pub fn supports_eub(&self) -> bool {
        !matches!(self, ExynosChip::Unknown(_))
    }

    /// True when ChimeraTool's documented "Change to EUB from ODIN" path
    /// works (no test-point required).
    pub fn supports_no_testpoint(&self) -> bool {
        matches!(self,
              ExynosChip::Exynos3830_850
            | ExynosChip::Exynos9630_980
            | ExynosChip::Exynos980)
    }
}

/// A device session in EUB mode — opaque handle owned by the operations
/// layer.
#[derive(Debug)]
pub struct EubSession {
    pub chip: ExynosChip,
    pub udid: Option<String>,
}

impl EubSession {
    /// Open a new EUB session. Stub — real implementation requires the
    /// UsbDk driver on Windows or libusb on macOS / Linux to enumerate
    /// the EUB endpoint at VID 0x04E8 PID 0x685D.
    pub fn open(chip: ExynosChip, udid: Option<String>) -> Result<Self> {
        Ok(Self { chip, udid })
    }
}

/// The set of procedures available once a device is in EUB mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EubProcedure {
    FactoryReset,
    StoreBackup,
    RestoreBackup,
    UnlockBootloader,
    RelockBootloader,
    CscChange,
    RepairSerial,
    RemoveFmm,
    RemoveFrp,
    KnoxguardRemove,
    SetKnoxguardState,
    RemoveLostMode,
    RemoveCommonCriteria,
    RemoveMdm,
    RemoveWarnings,
    RestoreSecurityBackup,
    ReadCodes,
    NetworkFactoryReset,
    NetworkRepair,
    PatchCert,
    RepairImei,
    RepairBoot,
    DemoRemove,
    ChangeToEub,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chip_display_works() {
        assert!(ExynosChip::Exynos8825_1280.display().contains("1280"));
        assert!(ExynosChip::Unknown(0).display().contains("unknown"));
    }
    #[test]
    fn known_chips_support_eub() {
        assert!(ExynosChip::Exynos9610.supports_eub());
        assert!(!ExynosChip::Unknown(0).supports_eub());
    }
    #[test]
    fn no_testpoint_subset_correct() {
        assert!(ExynosChip::Exynos3830_850.supports_no_testpoint());
        assert!(!ExynosChip::Exynos7884B.supports_no_testpoint());
    }
}
