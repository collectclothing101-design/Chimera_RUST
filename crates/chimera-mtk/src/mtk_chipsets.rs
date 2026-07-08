// crates/chimera-mtk/src/chipset_db.rs
//
// Reference list of MediaTek SoCs supported by upstream tooling.
// Source: enumerated from MediaTek-built Download Agent binaries
// (./platform/MT****/ paths and chipset-ID check strings).
// 30 application processors. Companion chips (PMICs, RF, codecs)
// are intentionally excluded — they are not flashing targets.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MtkChipset {
    // — Wearable —
    Mt2601,  // Aster — Android Wear (A7 dual)

    // — Internal/legacy tablet —
    Mt0571,  // ARMv7 tablet/IoT platform

    // — 2011–2014 entry phone SoCs —
    Mt6571, Mt6572, Mt6573, Mt6575, Mt6580, Mt6582, Mt6589, Mt6592, Mt6595,

    // — 2015–2019 mid-range smartphone SoCs —
    Mt6735,  // A53 quad LTE
    Mt6752,  // A53 octa LTE
    Mt6765,  // Helio P35 / G35
    Mt6768,  // Helio P65 / G70 / G80 / G85
    Mt6795,  // Helio X10

    // — Dimensity 5G era —
    Mt6833,  // Dimensity 700
    Mt6853,  // Dimensity 720 / 800 / 820 family
    Mt6873,  // Dimensity 800 / 820
    Mt6877,  // Dimensity 900 / 920 / 930 / 1080
    Mt6885,  // Dimensity 1000 / 1000L / 1000+

    // — Tablet / Chromebook / IoT / STB / projector —
    Mt8127, Mt8135, Mt8163, Mt8168, Mt8172, Mt8173,
    Mt8518,  // Smart speaker
    Mt8590,  // Smart STB
    Mt8695,  // Smart projector

    Unknown(u16),
}

impl MtkChipset {
    pub const ALL: &'static [MtkChipset] = &[
        Self::Mt0571, Self::Mt2601,
        Self::Mt6571, Self::Mt6572, Self::Mt6573, Self::Mt6575,
        Self::Mt6580, Self::Mt6582, Self::Mt6589, Self::Mt6592, Self::Mt6595,
        Self::Mt6735, Self::Mt6752, Self::Mt6765, Self::Mt6768, Self::Mt6795,
        Self::Mt6833, Self::Mt6853, Self::Mt6873, Self::Mt6877, Self::Mt6885,
        Self::Mt8127, Self::Mt8135, Self::Mt8163, Self::Mt8168, Self::Mt8172,
        Self::Mt8173, Self::Mt8518, Self::Mt8590, Self::Mt8695,
    ];

    pub fn marketing_name(&self) -> &'static str {
        use MtkChipset::*;
        match self {
            Mt2601 => "Aster",
            Mt6765 => "Helio P35/G35",
            Mt6768 => "Helio P65/G70/G80/G85",
            Mt6795 => "Helio X10",
            Mt6833 => "Dimensity 700",
            Mt6853 => "Dimensity 720/800/820",
            Mt6873 => "Dimensity 800/820",
            Mt6877 => "Dimensity 900/920/930/1080",
            Mt6885 => "Dimensity 1000",
            _ => "",
        }
    }

    pub fn is_5g(&self) -> bool {
        matches!(self, Self::Mt6833 | Self::Mt6853 | Self::Mt6873
                       | Self::Mt6877 | Self::Mt6885)
    }
}

// USB Vendor IDs recognised as MediaTek BootROM hosts.
pub const MTK_VID:  u16 = 0x0E8D; // MediaTek (canonical)
pub const OPPO_VID: u16 = 0x22D9; // OPPO rebrand of MTK BootROM
