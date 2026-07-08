// chimera-devices/src/database.rs
// Device model database with supported operations per model


use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub brand: String,
    pub model: String,
    pub model_code: String,
    pub chipset: String,
    pub operations: Vec<String>,
}

pub struct DeviceDatabase {
    models: IndexMap<String, ModelEntry>,
}

impl DeviceDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            models: IndexMap::new(),
        };
        db.populate();
        db
    }

    fn populate(&mut self) {
        // Samsung Flagship
        self.add("samsung", "Samsung Galaxy S24 Ultra", "SM-S928B", "Snapdragon 8 Gen 3",
            &["get_info", "frp_remove", "repair_imei", "csc_change", "mdm_remove", "knox_guard_remove", "update_firmware", "read_codes"]);
        self.add("samsung", "Samsung Galaxy S23", "SM-S911B", "Snapdragon 8 Gen 2",
            &["get_info", "frp_remove", "repair_imei", "csc_change", "mdm_remove", "update_firmware", "read_codes"]);
        self.add("samsung", "Samsung Galaxy A54", "SM-A546B", "Exynos 1380",
            &["get_info", "frp_remove", "repair_imei", "csc_change", "update_firmware"]);
        self.add("samsung", "Samsung Galaxy A14", "SM-A145F", "Helio G80 (MTK)",
            &["get_info", "frp_remove", "repair_imei", "repair_imei_patch", "update_firmware"]);
        self.add("samsung", "Samsung Galaxy M14", "SM-M146B", "Exynos 1330",
            &["get_info", "frp_remove", "repair_imei", "update_firmware"]);
        
        // Xiaomi
        self.add("xiaomi", "Xiaomi 14", "2023PANKBN", "Snapdragon 8 Gen 3",
            &["get_info", "frp_remove", "factory_reset", "repair_imei", "update_firmware"]);
        self.add("xiaomi", "Xiaomi Redmi Note 13", "23090RA98G", "Helio G88 (MTK)",
            &["get_info", "frp_remove", "factory_reset", "repair_imei_patch", "update_firmware"]);
        self.add("xiaomi", "POCO X6 Pro", "23122PCD1G", "Dimensity 8300 Ultra (MTK)",
            &["get_info", "frp_remove", "factory_reset", "repair_imei_patch", "update_firmware"]);
        
        // Huawei
        self.add("huawei", "Huawei P60 Pro", "MNA-AL00", "Kirin 9000s",
            &["get_info", "frp_remove", "factory_reset", "remove_huawei_id", "repair_imei", "update_firmware"]);
        self.add("huawei", "Huawei Nova 11", "FOA-LX9", "Snapdragon 778G",
            &["get_info", "frp_remove", "factory_reset", "remove_huawei_id"]);
        
        // More Samsung
        self.add("samsung", "Samsung Galaxy A55 5G", "SM-A556B", "Exynos 1480",
            &["get_info", "frp_remove", "repair_imei", "csc_change", "update_firmware"]);
        self.add("samsung", "Samsung Galaxy F15 5G", "SM-E156B", "Dimensity 6100+ (MTK)",
            &["get_info", "frp_remove", "repair_imei_patch", "update_firmware"]);
        
        // OnePlus
        self.add("oneplus", "OnePlus 12", "CPH2573", "Snapdragon 8 Gen 3",
            &["get_info", "frp_remove", "factory_reset", "bootloader_unlock", "update_firmware"]);
        
        // Vivo
        self.add("vivo", "Vivo X100 Pro", "V2324A", "Dimensity 9300 (MTK)",
            &["get_info", "frp_remove", "factory_reset", "repair_imei_patch", "update_firmware"]);
        
        // OPPO/Realme
        self.add("oppo", "OPPO Find X7 Ultra", "PHZ110", "Snapdragon 8 Gen 3",
            &["get_info", "frp_remove", "factory_reset", "update_firmware"]);
        self.add("realme", "Realme 12 Pro+", "RMX3840", "Snapdragon 7s Gen 2",
            &["get_info", "frp_remove", "factory_reset", "update_firmware"]);
        
        // Nokia
        self.add("nokia", "Nokia G42 5G", "TA-1581", "Snapdragon 480+",
            &["get_info", "frp_remove", "factory_reset", "update_firmware"]);
        
        // Motorola
        self.add("motorola", "Motorola Edge 50 Pro", "PB1J0001IN", "Snapdragon 7 Gen 3",
            &["get_info", "frp_remove", "factory_reset", "bootloader_unlock", "repair_imei", "update_firmware"]);
        self.add("motorola", "Moto G84 5G", "XT2347-1", "Snapdragon 695",
            &["get_info", "frp_remove", "factory_reset", "bootloader_unlock", "update_firmware"]);
        
        // LG
        self.add("lg", "LG V60 ThinQ", "LMV600TM", "Snapdragon 865",
            &["get_info", "frp_remove", "factory_reset", "repair_imei", "update_firmware"]);
        
        // Sony
        self.add("sony", "Sony Xperia 1 VI", "XQ-EC54", "Snapdragon 8 Gen 3",
            &["get_info", "frp_remove", "factory_reset", "bootloader_unlock", "update_firmware"]);
        
        // TCL/Alcatel
        self.add("tcl", "TCL 50 SE", "T610K", "Helio G85 (MTK)",
            &["get_info", "frp_remove", "factory_reset", "update_firmware"]);
        
        // Infinix
        self.add("infinix", "Infinix Note 40 Pro", "X6851", "Helio G99 Ultra (MTK)",
            &["get_info", "frp_remove", "factory_reset", "repair_imei_patch", "update_firmware"]);
        
        // Tecno
        self.add("tecno", "Tecno POVA 6 Pro", "LH8n", "Dimensity 6080 (MTK)",
            &["get_info", "frp_remove", "factory_reset", "repair_imei_patch", "update_firmware"]);

        
        // ════════════════════════════════════════════════════════════════════
        //   LESSER-KNOWN BRANDS (from ChimeraTool database)
        // ════════════════════════════════════════════════════════════════════
        
        // ════════════════════════════════════════════════════════════════════
        //   COMPREHENSIVE SAMSUNG MODELS
        // ════════════════════════════════════════════════════════════════════
        let samsung_flagship = &["get_info", "frp_remove", "repair_imei", "csc_change", "mdm_remove", "knox_guard_remove", "update_firmware", "read_codes", "store_backup", "restore_backup", "remove_lost_mode", "remove_warnings"];
        let samsung_midrange = &["get_info", "frp_remove", "repair_imei", "csc_change", "update_firmware", "store_backup"];
        let samsung_budget = &["get_info", "frp_remove", "repair_imei_patch", "update_firmware"];

        // Galaxy S Series
        self.add("samsung", "Samsung Galaxy S25 Ultra", "SM-S938B", "Snapdragon 8 Elite", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S25+", "SM-S936B", "Snapdragon 8 Elite", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S25", "SM-S931B", "Snapdragon 8 Elite", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S24 FE", "SM-S721B", "Exynos 2400e", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S24+", "SM-S926B", "Snapdragon 8 Gen 3", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S24", "SM-S921B", "Exynos 2400", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S23 FE", "SM-S711B", "Exynos 2200", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S23+", "SM-S916B", "Snapdragon 8 Gen 2", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S22 Ultra", "SM-S908B", "Snapdragon 8 Gen 1", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S21 FE", "SM-G990B", "Exynos 2100", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S21 Ultra", "SM-G998B", "Exynos 2100", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S20 FE", "SM-G780F", "Exynos 990", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S20 Ultra", "SM-G988B", "Exynos 990", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S10+", "SM-G975F", "Exynos 9820", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S10", "SM-G973F", "Exynos 9820", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S9+", "SM-G965F", "Exynos 9810", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S9", "SM-G960F", "Exynos 9810", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S8+", "SM-G955F", "Exynos 8895", samsung_flagship);
        self.add("samsung", "Samsung Galaxy S8", "SM-G950F", "Exynos 8895", samsung_flagship);

        // Galaxy Note Series
        self.add("samsung", "Samsung Galaxy Note 20 Ultra", "SM-N986B", "Exynos 990", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Note 20", "SM-N980F", "Exynos 990", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Note 10+", "SM-N975F", "Exynos 9825", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Note 9", "SM-N960F", "Exynos 9810", samsung_flagship);

        // Galaxy Z Series
        self.add("samsung", "Samsung Galaxy Z Fold6", "SM-F956B", "Snapdragon 8 Gen 3", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Z Flip6", "SM-F746B", "Snapdragon 8 Gen 3", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Z Fold5", "SM-F946B", "Snapdragon 8 Gen 2", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Z Flip5", "SM-F731B", "Snapdragon 8 Gen 2", samsung_flagship);

        // Galaxy A Series
        self.add("samsung", "Samsung Galaxy A75 5G", "SM-A756B", "Exynos 1480", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A56 5G", "SM-A566B", "Exynos 1580", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A36 5G", "SM-A366B", "Snapdragon 6 Gen 3", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A35 5G", "SM-A346B", "Exynos 1380", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A25 5G", "SM-A256E", "Exynos 1280", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A15", "SM-A155F", "Helio G99", samsung_budget);
        self.add("samsung", "Samsung Galaxy A14 5G", "SM-A146B", "Dimensity 700", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A13", "SM-A135F", "Exynos 850", samsung_budget);
        self.add("samsung", "Samsung Galaxy A12", "SM-A125F", "Helio P35", samsung_budget);
        self.add("samsung", "Samsung Galaxy A11", "SM-A115F", "Snapdragon 665", samsung_budget);
        self.add("samsung", "Samsung Galaxy A10", "SM-A105F", "Exynos 7904", samsung_budget);
        self.add("samsung", "Samsung Galaxy A80", "SM-A805F", "Snapdragon 730", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A71", "SM-A715F", "Snapdragon 730", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A70", "SM-A705F", "Snapdragon 675", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A51", "SM-A515F", "Exynos 9611", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A50", "SM-A505F", "Exynos 9610", samsung_midrange);
        self.add("samsung", "Samsung Galaxy A30", "SM-A305F", "Exynos 7904", samsung_midrange);

        // Galaxy M Series
        self.add("samsung", "Samsung Galaxy M55 5G", "SM-M556B", "Snapdragon 7 Gen 1", samsung_midrange);
        self.add("samsung", "Samsung Galaxy M35 5G", "SM-M356B", "Exynos 1380", samsung_midrange);
        self.add("samsung", "Samsung Galaxy M15 5G", "SM-M156B", "Dimensity 6100+", samsung_budget);
        self.add("samsung", "Samsung Galaxy M14 5G", "SM-M146B", "Exynos 1330", samsung_budget);
        self.add("samsung", "Samsung Galaxy M13", "SM-M135F", "Helio G88", samsung_budget);
        self.add("samsung", "Samsung Galaxy M51", "SM-M515F", "Snapdragon 730", samsung_midrange);
        self.add("samsung", "Samsung Galaxy M31", "SM-M315F", "Exynos 9611", samsung_midrange);
        self.add("samsung", "Samsung Galaxy M21", "SM-M215F", "Exynos 9611", samsung_budget);

        // Galaxy F Series
        self.add("samsung", "Samsung Galaxy F55 5G", "SM-E556B", "Snapdragon 7 Gen 1", samsung_midrange);
        self.add("samsung", "Samsung Galaxy F34 5G", "SM-E346B", "Exynos 1280", samsung_midrange);
        self.add("samsung", "Samsung Galaxy F15 5G", "SM-E156B", "Dimensity 6100+", samsung_budget);
        self.add("samsung", "Samsung Galaxy F14 5G", "SM-E146B", "Exynos 1330", samsung_budget);

        // Galaxy Tab Series
        self.add("samsung", "Samsung Galaxy Tab S10 Ultra", "SM-X926B", "Dimensity 9300+", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Tab S10+", "SM-X826B", "Dimensity 9300+", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Tab S9 FE", "SM-X516B", "Exynos 1380", samsung_midrange);
        self.add("samsung", "Samsung Galaxy Tab S9 Ultra", "SM-X916B", "Snapdragon 8 Gen 2", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Tab S9", "SM-X716B", "Snapdragon 8 Gen 2", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Tab S8 Ultra", "SM-X906B", "Snapdragon 8 Gen 1", samsung_flagship);
        self.add("samsung", "Samsung Galaxy Tab A9", "SM-X110", "Helio G99", samsung_budget);

        // Galaxy J Series
        self.add("samsung", "Samsung Galaxy J7 Pro", "SM-J730F", "Exynos 7870", samsung_budget);
        self.add("samsung", "Samsung Galaxy J6", "SM-J600F", "Exynos 7870", samsung_budget);
        self.add("samsung", "Samsung Galaxy J4", "SM-J400F", "Exynos 7570", samsung_budget);

        // ════════════════════════════════════════════════════════════════════
        //   COMPREHENSIVE XIAOMI MODELS
        // ════════════════════════════════════════════════════════════════════
        let xiaomi_flagship = &["get_info", "frp_remove", "factory_reset", "repair_imei", "bootloader_unlock", "update_firmware", "store_backup", "network_factory_reset"];
        let xiaomi_midrange = &["get_info", "frp_remove", "factory_reset", "repair_imei", "bootloader_unlock", "update_firmware", "store_backup"];
        let xiaomi_budget = &["get_info", "frp_remove", "factory_reset", "repair_imei_patch", "update_firmware", "enable_diag_mode"];

        // Xiaomi Number Series
        self.add("xiaomi", "Xiaomi 15 Ultra", "2501BPN24C", "Snapdragon 8 Elite", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 15 Pro", "24129PN74C", "Snapdragon 8 Elite", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 14T Pro", "2407BPDN1G", "Dimensity 9200+", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 14 Ultra", "2402PN24C", "Snapdragon 8 Gen 3", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 14 Pro", "23116PN5BC", "Snapdragon 8 Gen 3", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 13T Pro", "2307FPN6DC", "Dimensity 9200+", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 13 Pro", "2210132C", "Snapdragon 8 Gen 2", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 12T Pro", "22081283C", "Snapdragon 8+ Gen 1", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 12 Pro", "nuwa", "Snapdragon 8 Gen 1", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 11 Pro", "mars", "Snapdragon 888", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 10 Pro", "cmi", "Snapdragon 865", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi 9T Pro", "raphael", "Snapdragon 855", xiaomi_flagship);

        // Redmi Note Series
        self.add("redmi", "Redmi Note 14 Pro+ 5G", "2412DPC22R", "Snapdragon 7s Gen 3", xiaomi_midrange);
        self.add("redmi", "Redmi Note 14 Pro 5G", "24090RA29G", "Dimensity 7300 Ultra", xiaomi_midrange);
        self.add("redmi", "Redmi Note 13 Pro+ 5G", "23127RAA3C", "Dimensity 7200 Ultra", xiaomi_midrange);
        self.add("redmi", "Redmi Note 13 Pro", "23127RA9OC", "Helio G99 Ultra", xiaomi_budget);
        self.add("redmi", "Redmi Note 12 Pro+ 5G", "22101316G", "Dimensity 1080", xiaomi_midrange);
        self.add("redmi", "Redmi Note 11 Pro 5G", "2201116TG", "Snapdragon 695", xiaomi_midrange);
        self.add("redmi", "Redmi Note 10 Pro", "sweet", "Snapdragon 732G", xiaomi_midrange);
        self.add("redmi", "Redmi Note 9 Pro", "joyeuse", "Snapdragon 720G", xiaomi_midrange);
        self.add("redmi", "Redmi Note 8 Pro", "begonia", "Helio G90T", xiaomi_budget);
        self.add("redmi", "Redmi Note 7 Pro", "violet", "Snapdragon 675", xiaomi_midrange);

        // Redmi Number Series
        self.add("redmi", "Redmi 13C", "air", "Helio G85", xiaomi_budget);
        self.add("redmi", "Redmi 12", "rain", "Helio G88", xiaomi_budget);
        self.add("redmi", "Redmi 11 Prime", "fire", "Helio G99", xiaomi_budget);
        self.add("redmi", "Redmi 10", "fog", "Snapdragon 680", xiaomi_budget);
        self.add("redmi", "Redmi 9A", "dandelion", "Helio G25", xiaomi_budget);

        // POCO Series
        self.add("poco", "POCO F6 Pro", "vermeer", "Snapdragon 8 Gen 2", xiaomi_flagship);
        self.add("poco", "POCO F6", "peridot", "Snapdragon 8s Gen 3", xiaomi_flagship);
        self.add("poco", "POCO X6 Pro 5G", "duchamp", "Dimensity 8300 Ultra", xiaomi_midrange);
        self.add("poco", "POCO X6 5G", "garnet", "Snapdragon 7s Gen 2", xiaomi_midrange);
        self.add("poco", "POCO F5 Pro", "mondrian", "Snapdragon 8+ Gen 1", xiaomi_flagship);
        self.add("poco", "POCO F5", "marble", "Snapdragon 7+ Gen 2", xiaomi_midrange);
        self.add("poco", "POCO M5", "fire", "Helio G99", xiaomi_budget);
        self.add("poco", "POCO X3 Pro", "vayu", "Snapdragon 860", xiaomi_midrange);
        self.add("poco", "POCO X3 NFC", "surya", "Snapdragon 732G", xiaomi_midrange);

        // Mix Series
        self.add("xiaomi", "Xiaomi Mix Fold 4", "goku", "Snapdragon 8 Gen 3", xiaomi_flagship);
        self.add("xiaomi", "Xiaomi Mix Fold 3", "babylon", "Snapdragon 8 Gen 2", xiaomi_flagship);

        // Redmi K Series
        self.add("redmi", "Redmi K70 Pro", "vermeer", "Snapdragon 8 Gen 3", xiaomi_flagship);
        self.add("redmi", "Redmi K60 Pro", "socrates", "Snapdragon 8 Gen 2", xiaomi_flagship);
        self.add("redmi", "Redmi K50 Pro", "marble", "Snapdragon 8 Gen 1", xiaomi_flagship);
        self.add("redmi", "Redmi K40 Pro", "star", "Snapdragon 888", xiaomi_flagship);

let lesser_ops = &["get_info", "frp_remove", "factory_reset", "update_firmware"];
        let lesser_ops_plus = &["get_info", "frp_remove", "factory_reset", "repair_imei_patch", "update_firmware"];

        // ── Blackview ──────────────────────────────────────────────────
        self.add("blackview", "Blackview BV8800", "BV8800", "Helio G96", lesser_ops_plus);
        self.add("blackview", "Blackview BV9200", "BV9200", "Helio G99", lesser_ops_plus);
        self.add("blackview", "Blackview BV9300 Pro", "BV9300Pro", "Helio G99", lesser_ops_plus);
        self.add("blackview", "Blackview BV7200", "BV7200", "Helio G85", lesser_ops);
        self.add("blackview", "Blackview Tab 16", "tab16", "Helio G88", lesser_ops);
        self.add("blackview", "Blackview Tab 80", "tab80", "Helio G85", lesser_ops);
        self.add("blackview", "Blackview Wave 8C", "wave8c", "Helio G85", lesser_ops_plus);
        self.add("blackview", "Blackview Wave 8", "wave8", "Helio G85", lesser_ops);
        self.add("blackview", "Blackview Zeno 1", "zeno1", "Helio G85", lesser_ops);

        // ── Tecno ─────────────────────────────────────────────────────
        self.add("tecno", "Tecno Pova 6 Pro", "LH8n", "Dimensity 6080", lesser_ops_plus);
        self.add("tecno", "Tecno Pova 5 Pro", "LG7n", "Dimensity 6020", lesser_ops);
        self.add("tecno", "Tecno Pova 3", "LF7n", "Helio G85", lesser_ops_plus);
        self.add("tecno", "Tecno Camon 30 Premier", "CPH2579", "Dimensity 8200", lesser_ops_plus);
        self.add("tecno", "Tecno Camon 30", "CI7n", "Helio G99 Ultra", lesser_ops_plus);
        self.add("tecno", "Tecno Camon 20 Pro", "CK7", "Helio G85", lesser_ops);
        self.add("tecno", "Tecno Spark 20 Pro", "CK9", "Helio G99", lesser_ops);
        self.add("tecno", "Tecno Spark 10 Pro", "BF8", "Helio G85", lesser_ops);
        self.add("tecno", "Tecno Pop 8", "BE8", "Helio G36", lesser_ops);

        // ── Infinix ───────────────────────────────────────────────────
        self.add("infinix", "Infinix GT 20 Pro", "X6725", "Dimensity 8200", lesser_ops_plus);
        self.add("infinix", "Infinix Note 40 Pro", "X6851", "Helio G99 Ultra", lesser_ops_plus);
        self.add("infinix", "Infinix Hot 50 5G", "X6720B", "Dimensity 6300", lesser_ops);
        self.add("infinix", "Infinix Hot 40 Pro", "X6725B", "Helio G99", lesser_ops);
        self.add("infinix", "Infinix Hot 30 Play", "X6835", "Helio G85", lesser_ops_plus);
        self.add("infinix", "Infinix Smart 8", "X6525", "Helio G25", lesser_ops);
        self.add("infinix", "Infinix Note 12", "X670", "Helio G96", lesser_ops_plus);
        self.add("infinix", "Infinix Note 11S", "X698", "Helio G96", lesser_ops_plus);

        // ── Doogee ────────────────────────────────────────────────────
        self.add("doogee", "Doogee S98 Pro", "S98Pro", "Dimensity 900", lesser_ops_plus);
        self.add("doogee", "Doogee S89 Pro", "S89Pro", "Helio G85", lesser_ops);
        self.add("doogee", "Doogee N40 Pro", "N40Pro", "Helio G85", lesser_ops);
        self.add("doogee", "Doogee X96 Pro", "X96Pro", "Helio G85", lesser_ops);
        self.add("doogee", "Doogee Y9 Pro", "Y9Pro", "Helio G85", lesser_ops);

        // ── Ulefone ───────────────────────────────────────────────────
        self.add("ulefone", "Ulefone Armor 28 Ultra", "Armor28Ultra", "Dimensity 9300+", lesser_ops_plus);
        self.add("ulefone", "Ulefone Armor 28 Pro", "Armor28Pro", "Dimensity 8200", lesser_ops_plus);
        self.add("ulefone", "Ulefone Armor 25T Pro", "Armor25TPro", "Dimensity 8200", lesser_ops_plus);
        self.add("ulefone", "Ulefone Armor 24", "Armor24", "Helio G99", lesser_ops);
        self.add("ulefone", "Ulefone Armor X12 Pro", "ArmorX12Pro", "Helio G85", lesser_ops);
        self.add("ulefone", "Ulefone Note 17 Pro", "Note17Pro", "Helio G99", lesser_ops);

        // ── Oukitel ───────────────────────────────────────────────────
        self.add("oukitel", "Oukitel WP38", "WP38", "Dimensity 6300", lesser_ops_plus);
        self.add("oukitel", "Oukitel WP36 Pro", "WP36Pro", "Helio G85", lesser_ops);
        self.add("oukitel", "Oukitel WP30 Pro", "WP30Pro", "Helio G99", lesser_ops);
        self.add("oukitel", "Oukitel K15 Pro", "K15Pro", "Helio G85", lesser_ops);
        self.add("oukitel", "Oukitel C32", "C32", "Helio G85", lesser_ops);

        // ── Umidigi ───────────────────────────────────────────────────
        self.add("umidigi", "Umidigi Bison 3", "Bison3", "Helio G99", lesser_ops);
        self.add("umidigi", "Umidigi Bison 2 Pro", "Bison2Pro", "Helio G85", lesser_ops);
        self.add("umidigi", "Umidigi A15 Pro", "A15Pro", "Helio G85", lesser_ops);
        self.add("umidigi", "Umidigi A13 Pro", "A13Pro", "Helio G99", lesser_ops);
        self.add("umidigi", "Umidigi Power 8", "Power8", "Helio G85", lesser_ops);

        // ── Cubot ─────────────────────────────────────────────────────
        self.add("cubot", "Cubot X90", "X90", "Dimensity 7200", lesser_ops_plus);
        self.add("cubot", "Cubot KingKong 8", "KingKong8", "Helio G85", lesser_ops);
        self.add("cubot", "Cubot P80", "P80", "Helio G85", lesser_ops);

        // ── Honor ─────────────────────────────────────────────────────
        self.add("honor", "Honor Magic6 Pro", "BVL-AN16", "Snapdragon 8 Gen 3", lesser_ops_plus);
        self.add("honor", "Honor Magic5 Pro", "BVL-AN16", "Snapdragon 8 Gen 2", lesser_ops_plus);
        self.add("honor", "Honor 200 Pro", "ELP-AN00", "Snapdragon 8s Gen 3", lesser_ops_plus);
        self.add("honor", "Honor 100 Pro", "REP-AN00", "Snapdragon 8 Gen 2", lesser_ops_plus);
        self.add("honor", "Honor X9c", "LLY-LX1", "Snapdragon 685", lesser_ops);
        self.add("honor", "Honor X7b", "MGA-LX1", "Snapdragon 685", lesser_ops);

        // ── Hisense ───────────────────────────────────────────────────
        self.add("hisense", "Hisense U950 Pro", "U950Pro", "Snapdragon 8 Gen 2", lesser_ops_plus);
        self.add("hisense", "Hisense U70 Pro", "U70Pro", "Snapdragon 778G", lesser_ops);
        self.add("hisense", "Hisense F50", "F50", "Dimensity 800", lesser_ops_plus);

        // ── ZTE ───────────────────────────────────────────────────────
        self.add("zte", "ZTE Axon 60 Ultra", "Axon60Ultra", "Snapdragon 8 Gen 3", lesser_ops_plus);
        self.add("zte", "ZTE Blade V50 5G", "BladeV50-5G", "Dimensity 6020", lesser_ops);
        self.add("zte", "ZTE Nubia Z60 Ultra", "NubiaZ60Ultra", "Snapdragon 8 Gen 3", lesser_ops_plus);
        self.add("zte", "ZTE Nubia Red Magic 9S Pro", "RedMagic9SPro", "Snapdragon 8 Gen 3", lesser_ops_plus);

        // ── Realme ────────────────────────────────────────────────────
        self.add("realme", "Realme GT 6 Pro", "RMX3851", "Snapdragon 8s Gen 3", lesser_ops_plus);
        self.add("realme", "Realme GT 5 Pro", "RMX3888", "Snapdragon 8 Gen 3", lesser_ops_plus);
        self.add("realme", "Realme 13 Pro+", "RMX3860", "Snapdragon 7s Gen 2", lesser_ops);
        self.add("realme", "Realme 12 Pro+", "RMX3841", "Snapdragon 7s Gen 2", lesser_ops);
        self.add("realme", "Realme C67 5G", "RMX3771", "Dimensity 6100+", lesser_ops);
        self.add("realme", "Realme Narzo 70 Pro", "RMX3771", "Dimensity 7050", lesser_ops);

        // ── Vivo ──────────────────────────────────────────────────────
        self.add("vivo", "Vivo X200 Pro", "V2405A", "Dimensity 9400", lesser_ops_plus);
        self.add("vivo", "Vivo X100 Pro", "V2324A", "Dimensity 9300", lesser_ops_plus);
        self.add("vivo", "Vivo V40", "V2348", "Snapdragon 7 Gen 3", lesser_ops);
        self.add("vivo", "Vivo Y36 5G", "V2248", "Dimensity 6020", lesser_ops_plus);

        // ── OPPO ──────────────────────────────────────────────────────
        self.add("oppo", "OPPO Find X7 Ultra", "PHZ110", "Snapdragon 8 Gen 3", lesser_ops_plus);
        self.add("oppo", "OPPO Reno11 F 5G", "CPH2603", "Dimensity 7050", lesser_ops);
        self.add("oppo", "OPPO A98 5G", "CPH2523", "Snapdragon 695", lesser_ops);
        self.add("oppo", "OPPO K11", "PHV110", "Snapdragon 782G", lesser_ops);

        // ── Motorola ──────────────────────────────────────────────────
        self.add("motorola", "Motorola Edge 50 Ultra", "XT2401-4", "Snapdragon 8s Gen 3", lesser_ops_plus);
        self.add("motorola", "Motorola ThinkPhone", "XT2301-5", "Snapdragon 8+ Gen 1", lesser_ops_plus);
        self.add("motorola", "Moto G84 5G", "XT2347-1", "Snapdragon 695", lesser_ops);
        self.add("motorola", "Moto G73 5G", "XT2237-4", "Dimensity 930", lesser_ops);
        self.add("motorola", "Moto G54 5G", "XT2331-3", "Dimensity 7020", lesser_ops);

        // ── LG ────────────────────────────────────────────────────────
        self.add("lg", "LG V60 ThinQ", "LMV600TM", "Snapdragon 865", lesser_ops_plus);
        self.add("lg", "LG V50 ThinQ", "LMV500EM", "Snapdragon 855", lesser_ops_plus);
        self.add("lg", "LG Velvet", "LMG910EM", "Snapdragon 765G", lesser_ops);
        self.add("lg", "LG Wing", "LM-F100L", "Snapdragon 765G", lesser_ops);

        // ── Sony ──────────────────────────────────────────────────────
        self.add("sony", "Sony Xperia 1 VI", "XQ-EC54", "Snapdragon 8 Gen 3", lesser_ops_plus);
        self.add("sony", "Sony Xperia 5 V", "XQ-DC72", "Snapdragon 8 Gen 2", lesser_ops_plus);
        self.add("sony", "Sony Xperia 1 V", "XQ-DC54", "Snapdragon 8 Gen 2", lesser_ops_plus);

        // ── Nokia ─────────────────────────────────────────────────────
        self.add("nokia", "Nokia G60 5G", "TA-1588", "Snapdragon 695", lesser_ops);
        self.add("nokia", "Nokia G42 5G", "TA-1581", "Snapdragon 480+", lesser_ops);
        self.add("nokia", "Nokia X30 5G", "TA-1484", "Snapdragon 695", lesser_ops);

        // ── TCL ───────────────────────────────────────────────────────
        self.add("tcl", "TCL 50 XL 5G", "T781H", "Dimensity 6100+", lesser_ops);
        self.add("tcl", "TCL 50 SE", "T610K", "Helio G85", lesser_ops);
        self.add("tcl", "TCL 40 SE", "T610K", "Helio G85", lesser_ops);

        // ── Nothing ───────────────────────────────────────────────────
        self.add("nothing", "Nothing Phone (2a) Plus", "A142", "Dimensity 7350 Pro", lesser_ops);
        self.add("nothing", "Nothing Phone (2a)", "A142", "Dimensity 7200 Pro", lesser_ops);
        self.add("nothing", "Nothing Phone (2)", "A065", "Snapdragon 8+ Gen 1", lesser_ops_plus);

        // ── Blackberry ────────────────────────────────────────────────
        self.add("blackberry", "BlackBerry KEY2", "BBF100-1", "Snapdragon 660", lesser_ops);
        self.add("blackberry", "BlackBerry KEYone", "BBB100-1", "Snapdragon 625", lesser_ops);

        // ── Coolpad ───────────────────────────────────────────────────
        self.add("coolpad", "Coolpad Cool 50", "CP07", "Helio G85", lesser_ops);
        self.add("coolpad", "Coolpad Cool 30", "CP05", "Helio G85", lesser_ops);

        // ── Fairphone ─────────────────────────────────────────────────
        self.add("fairphone", "Fairphone 5", "FP5", "Snapdragon 778G", lesser_ops_plus);
        self.add("fairphone", "Fairphone 4", "FP4", "Snapdragon 750G", lesser_ops);

        // ── CAT ───────────────────────────────────────────────────────
        self.add("cat", "CAT S75", "S75", "Snapdragon 6 Gen 1", lesser_ops);
        self.add("cat", "CAT S62 Pro", "S62Pro", "Snapdragon 660", lesser_ops);

        // ── Meizu ─────────────────────────────────────────────────────
        self.add("meizu", "Meizu 21 Pro", "M461H", "Snapdragon 8 Gen 3", lesser_ops_plus);
        self.add("meizu", "Meizu 20 Pro", "M391H", "Snapdragon 8 Gen 2", lesser_ops_plus);

        // ── iQOO ──────────────────────────────────────────────────────
        self.add("iqoo", "iQOO 12", "V2354A", "Snapdragon 8 Gen 3", lesser_ops_plus);

// ── Apple iPhone 15 series ─────────────────────────────────────────
        let apple_ops = &["get_info", "flash_ipsw", "check_icloud", "bypass_icloud",
                          "icloud_wipe", "remove_passcode", "enter_recovery", "exit_recovery",
                          "network_unlock"];
        self.add("apple", "iPhone 15",          "iPhone15,4", "Apple A16 Bionic", apple_ops);
        self.add("apple", "iPhone 15 Plus",     "iPhone15,5", "Apple A16 Bionic", apple_ops);
        self.add("apple", "iPhone 15 Pro",      "iPhone16,1", "Apple A17 Pro",    apple_ops);
        self.add("apple", "iPhone 15 Pro Max",  "iPhone16,2", "Apple A17 Pro",    apple_ops);

        // ── Apple iPhone 16 series ─────────────────────────────────────────
        self.add("apple", "iPhone 16",          "iPhone17,3", "Apple A18",        apple_ops);
        self.add("apple", "iPhone 16 Plus",     "iPhone17,4", "Apple A18",        apple_ops);
        self.add("apple", "iPhone 16 Pro",      "iPhone17,1", "Apple A18 Pro",    apple_ops);
        self.add("apple", "iPhone 16 Pro Max",  "iPhone17,2", "Apple A18 Pro",    apple_ops);
        self.add("apple", "iPhone 16e",         "iPhone17,5", "Apple A16 Bionic", apple_ops);

        // ── Apple iPhone 17 series (2025) ─────────────────────────────────
        self.add("apple", "iPhone 17 Air",      "iPhone18,5", "Apple A19",        apple_ops);
        self.add("apple", "iPhone 17",          "iPhone18,3", "Apple A19",        apple_ops);
        self.add("apple", "iPhone 17",          "iPhone18,4", "Apple A19",        apple_ops);
        self.add("apple", "iPhone 17 Pro",      "iPhone18,1", "Apple A19 Pro",    apple_ops);
        self.add("apple", "iPhone 17 Pro Max",  "iPhone18,2", "Apple A19 Pro",    apple_ops);

        // ── Apple iPad Air 16/17 & iPad Pro ────────────────────────────────
        self.add("apple", "iPad Air 11-inch (M3)",       "iPad15,4", "Apple M3", apple_ops);
        self.add("apple", "iPad Air 13-inch (M3)",       "iPad15,5", "Apple M3", apple_ops);
        self.add("apple", "iPad Air 11-inch M4 (2025)",  "iPad16,3", "Apple M4", apple_ops);
        self.add("apple", "iPad Air 13-inch M4 (2025)",  "iPad16,4", "Apple M4", apple_ops);
        self.add("apple", "iPad Pro 11-inch (M4)",       "iPad16,1", "Apple M4", apple_ops);
        self.add("apple", "iPad Pro 13-inch (M4)",       "iPad16,2", "Apple M4", apple_ops);
        self.add("apple", "iPad mini 7",                 "iPad16,5", "Apple A17 Pro", apple_ops);
        self.add("apple", "iPad (11th gen)",             "iPad16,7", "Apple A16", apple_ops);

        // ── Apple legacy (checkm8 / common unlock targets) ─────────────────
        let legacy_ops = &["get_info", "flash_ipsw", "check_icloud", "bypass_icloud",
                           "remove_passcode", "enter_recovery", "exit_recovery"];
        self.add("apple", "iPhone X",          "iPhone10,3", "Apple A11 Bionic", legacy_ops);
        self.add("apple", "iPhone XS",         "iPhone11,2", "Apple A12 Bionic", legacy_ops);
        self.add("apple", "iPhone XS Max",     "iPhone11,6", "Apple A12 Bionic", legacy_ops);
        self.add("apple", "iPhone XR",         "iPhone11,8", "Apple A12 Bionic", legacy_ops);
        self.add("apple", "iPhone 11",         "iPhone12,1", "Apple A13 Bionic", legacy_ops);
        self.add("apple", "iPhone 11 Pro",     "iPhone12,3", "Apple A13 Bionic", legacy_ops);
        self.add("apple", "iPhone 11 Pro Max", "iPhone12,5", "Apple A13 Bionic", legacy_ops);
        self.add("apple", "iPhone SE (2nd gen)","iPhone12,8", "Apple A13 Bionic", legacy_ops);
        self.add("apple", "iPhone 12",         "iPhone13,2", "Apple A14 Bionic", legacy_ops);
        self.add("apple", "iPhone 12 Pro",     "iPhone13,3", "Apple A14 Bionic", legacy_ops);
        self.add("apple", "iPhone 12 Pro Max", "iPhone13,4", "Apple A14 Bionic", legacy_ops);
        self.add("apple", "iPhone 13",         "iPhone14,5", "Apple A15 Bionic", legacy_ops);
        self.add("apple", "iPhone 13 Pro",     "iPhone14,2", "Apple A15 Bionic", legacy_ops);
        self.add("apple", "iPhone 13 Pro Max", "iPhone14,3", "Apple A15 Bionic", legacy_ops);
        self.add("apple", "iPhone SE (3rd gen)","iPhone14,6", "Apple A15 Bionic", legacy_ops);
        self.add("apple", "iPhone 14",         "iPhone14,7", "Apple A15 Bionic", legacy_ops);
        self.add("apple", "iPhone 14 Plus",    "iPhone14,8", "Apple A15 Bionic", legacy_ops);
        self.add("apple", "iPhone 14 Pro",     "iPhone15,2", "Apple A16 Bionic", legacy_ops);
        self.add("apple", "iPhone 14 Pro Max", "iPhone15,3", "Apple A16 Bionic", legacy_ops);
    }

    fn add(&mut self, brand: &str, model: &str, code: &str, chipset: &str, ops: &[&str]) {
        let entry = ModelEntry {
            brand: brand.to_string(),
            model: model.to_string(),
            model_code: code.to_string(),
            chipset: chipset.to_string(),
            operations: ops.iter().map(|s| s.to_string()).collect(),
        };
        self.models.insert(code.to_lowercase(), entry);
    }

    pub fn find_by_code(&self, code: &str) -> Option<&ModelEntry> {
        self.models.get(&code.to_lowercase())
    }

    pub fn find_by_model(&self, model: &str) -> Vec<&ModelEntry> {
        let model_lower = model.to_lowercase();
        self.models.values()
            .filter(|e| e.model.to_lowercase().contains(&model_lower))
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&ModelEntry> {
        let q = query.to_lowercase();
        self.models.values()
            .filter(|e| {
                e.model.to_lowercase().contains(&q)
                    || e.model_code.to_lowercase().contains(&q)
                    || e.brand.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn all_brands(&self) -> Vec<String> {
        let mut brands: Vec<String> = self.models.values()
            .map(|e| e.brand.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        brands.sort();
        brands
    }

    pub fn count(&self) -> usize {
        self.models.len()
    }
}

impl Default for DeviceDatabase {
    fn default() -> Self {
        Self::new()
    }
}