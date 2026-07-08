// chimera-samsung/src/lib.rs
// Samsung device support: ODIN protocol, EFS operations, FRP, IMEI, MDM, Knox,
// + Exynos USB Boot (EUB), Read Codes, CSC database, Knox / Knoxguard / CC,
// + Qualcomm programmer file analyser.

pub mod odin;
pub mod efs;
pub mod operations;
pub mod frp;
pub mod imei;
pub mod certificate;
pub mod firmware;
pub mod mdm;
pub mod mtk;
pub mod read_codes;
pub mod csc;
pub mod eub;
pub mod knox;
pub mod programmer;

pub use operations::SamsungOperations;
pub use odin::{OdinClient, OdinSession};
pub use read_codes::{LockCodes, parse_at_response};
pub use csc::{CscCode, CscChangeRequest, CscChangeResult,
              lookup_csc, search_csc, validate_csc,
              all_csc_codes, csc_database_len};
pub use eub::{ExynosChip, EubSession, EubProcedure};
pub use knox::{KnoxStatus, KnoxWarrantyState, KnoxguardState,
               parse_getprop, require_warranty_untripped};
pub use programmer::{ProgrammerInfo, StorageType, FileFormat,
                     analyse_file, analyse_directory};
