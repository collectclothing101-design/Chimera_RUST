//! StageNow profile generator.
//!
//! StageNow is Zebra's free Windows tool that builds barcode / NFC / USB
//! profiles for first-boot provisioning. This module produces the same
//! XML wire-format StageNow generates so a profile can be authored
//! programmatically and then:
//!   - rendered to a barcode (via `chimera_utils::QrCodeGenerator` or
//!     a PDF417 encoder)
//!   - dropped on an SD card for boot-time pickup
//!   - pushed through USB to a paired Stager service
//!
//! Profile types supported here:
//! Wi-Fi, OEM-config (MX feature toggles), App install/uninstall,
//! Factory reset, Lock-task / kiosk mode, EMM de-enrollment, build-prop
//! tweaks (debug builds only), OTA update.

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageNowProfile {
    pub name:     String,
    pub author:   String,
    pub version:  String,
    pub stages:   Vec<Stage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stage {
    WifiConfig(WifiConfig),
    AppInstall   { url: String, package_name: String, grant_permissions: bool },
    AppUninstall { package_name: String },
    Mx(MxToggles),
    FactoryReset { preserve_sd_card: bool },
    LockTaskMode { package_name: String },
    DeenrollEmm  { current_owner_pkg: String, justification: String },
    SetProperty  { key: String, value: String },
    OtaUpdate    { url: String, sha256: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiConfig {
    pub ssid:       String,
    pub security:   WifiSecurity,
    pub hidden:     bool,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WifiSecurity {
    Open,
    Wep     { key: String },
    WpaPsk  { password: String },
    Wpa2Psk { password: String },
    Wpa3Sae { password: String },
    Wpa2Eap {
        eap_method:         String,
        phase2:             String,
        identity:           String,
        anonymous_identity: Option<String>,
        password:           Option<String>,
        ca_cert_pem:        Option<String>,
        client_cert_pem:    Option<String>,
        client_key_pem:     Option<String>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MxToggles {
    pub usb_enabled:               Option<bool>,
    pub bluetooth_enabled:         Option<bool>,
    pub wifi_enabled:              Option<bool>,
    pub gps_enabled:               Option<bool>,
    pub nfc_enabled:               Option<bool>,
    pub camera_enabled:            Option<bool>,
    pub mic_enabled:               Option<bool>,
    pub scanner_enabled:           Option<bool>,
    pub sd_card_enabled:           Option<bool>,
    pub usb_mass_storage_enabled:  Option<bool>,
    pub adb_over_usb:              Option<bool>,
    pub adb_over_network:          Option<bool>,
    pub developer_options_enabled: Option<bool>,
    pub safe_mode_disabled:        Option<bool>,
    pub factory_reset_disabled:    Option<bool>,
    pub install_unknown_sources:   Option<bool>,
    pub screen_capture_enabled:    Option<bool>,
}

impl StageNowProfile {
    pub fn to_xml(&self) -> String {
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        out.push_str(&format!(
            "<wap-provisioningdoc name=\"{}\" version=\"{}\" author=\"{}\">\n",
            xml_escape(&self.name), xml_escape(&self.version), xml_escape(&self.author)));
        out.push_str("  <characteristic version=\"9.4\" type=\"Profile\">\n");
        for (i, st) in self.stages.iter().enumerate() {
            out.push_str(&format!("    <!-- stage {} -->\n", i + 1));
            out.push_str(&render_stage(st));
        }
        out.push_str("  </characteristic>\n");
        out.push_str("</wap-provisioningdoc>\n");
        out
    }

    /// 16-hex-char SHA-256-truncated id, stable for the rendered XML.
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.to_xml().as_bytes());
        hex::encode(&h.finalize()[..8])
    }
}

fn render_stage(stage: &Stage) -> String {
    match stage {
        Stage::WifiConfig(w) => {
            let mut s = String::new();
            s.push_str("    <characteristic type=\"WifiMgr\">\n");
            s.push_str(&parm("SSID", &w.ssid));
            s.push_str(&parm("Hidden", &w.hidden.to_string()));
            match &w.security {
                WifiSecurity::Open                      => s.push_str(&parm("Security", "None")),
                WifiSecurity::Wep { key }               => {
                    s.push_str(&parm("Security", "WEP"));
                    s.push_str(&parm("WepKey", key));
                }
                WifiSecurity::WpaPsk  { password }
                | WifiSecurity::Wpa2Psk { password }    => {
                    s.push_str(&parm("Security", "WPA2-PSK"));
                    s.push_str(&parm("PSK", password));
                }
                WifiSecurity::Wpa3Sae { password }      => {
                    s.push_str(&parm("Security", "WPA3-SAE"));
                    s.push_str(&parm("SAE", password));
                }
                WifiSecurity::Wpa2Eap {
                    eap_method, phase2, identity, anonymous_identity, password,
                    ca_cert_pem, client_cert_pem, client_key_pem
                } => {
                    s.push_str(&parm("Security",  "WPA2-EAP"));
                    s.push_str(&parm("EAPMethod", eap_method));
                    s.push_str(&parm("Phase2",    phase2));
                    s.push_str(&parm("Identity",  identity));
                    if let Some(a) = anonymous_identity { s.push_str(&parm("AnonymousIdentity", a)); }
                    if let Some(p) = password           { s.push_str(&parm("Password", p)); }
                    if let Some(p) = ca_cert_pem        { s.push_str(&parm("CACert", p)); }
                    if let Some(p) = client_cert_pem    { s.push_str(&parm("ClientCert", p)); }
                    if let Some(p) = client_key_pem     { s.push_str(&parm("ClientKey", p)); }
                }
            }
            if let Some(h) = &w.proxy_host { s.push_str(&parm("ProxyHost", h)); }
            if let Some(p) = w.proxy_port  { s.push_str(&parm("ProxyPort", &p.to_string())); }
            s.push_str("    </characteristic>\n");
            s
        }
        Stage::AppInstall { url, package_name, grant_permissions } => {
            let mut s = String::new();
            s.push_str("    <characteristic type=\"AppMgr\">\n");
            s.push_str(&parm("Action", "Install"));
            s.push_str(&parm("PackageName", package_name));
            s.push_str(&parm("PackageURL", url));
            s.push_str(&parm("GrantRuntimePermissions", &grant_permissions.to_string()));
            s.push_str("    </characteristic>\n");
            s
        }
        Stage::AppUninstall { package_name } => {
            let mut s = String::new();
            s.push_str("    <characteristic type=\"AppMgr\">\n");
            s.push_str(&parm("Action", "Uninstall"));
            s.push_str(&parm("PackageName", package_name));
            s.push_str("    </characteristic>\n");
            s
        }
        Stage::Mx(t) => {
            let mut s = String::new();
            s.push_str("    <characteristic type=\"FeatureMgr\">\n");
            macro_rules! emit {
                ($field:ident, $key:literal) => {
                    if let Some(v) = t.$field {
                        s.push_str(&parm($key, if v { "Enable" } else { "Disable" }));
                    }
                };
            }
            emit!(usb_enabled,              "Usb");
            emit!(bluetooth_enabled,        "Bluetooth");
            emit!(wifi_enabled,             "Wifi");
            emit!(gps_enabled,              "Gps");
            emit!(nfc_enabled,              "Nfc");
            emit!(camera_enabled,           "Camera");
            emit!(mic_enabled,              "Microphone");
            emit!(scanner_enabled,          "Scanner");
            emit!(sd_card_enabled,          "SDCard");
            emit!(usb_mass_storage_enabled, "UsbMassStorage");
            emit!(adb_over_usb,             "AdbOverUsb");
            emit!(adb_over_network,         "AdbOverNetwork");
            emit!(developer_options_enabled,"DeveloperOptions");
            emit!(safe_mode_disabled,       "SafeModeDisabled");
            emit!(factory_reset_disabled,   "FactoryResetDisabled");
            emit!(install_unknown_sources,  "InstallUnknownSources");
            emit!(screen_capture_enabled,   "ScreenCapture");
            s.push_str("    </characteristic>\n");
            s
        }
        Stage::FactoryReset { preserve_sd_card } => {
            let mut s = String::new();
            s.push_str("    <characteristic type=\"PowerMgr\">\n");
            s.push_str(&parm("Action", "FactoryReset"));
            s.push_str(&parm("PreserveSDCard", &preserve_sd_card.to_string()));
            s.push_str("    </characteristic>\n");
            s
        }
        Stage::LockTaskMode { package_name } => {
            let mut s = String::new();
            s.push_str("    <characteristic type=\"LockTaskMgr\">\n");
            s.push_str(&parm("Action", "Lock"));
            s.push_str(&parm("PackageName", package_name));
            s.push_str("    </characteristic>\n");
            s
        }
        Stage::DeenrollEmm { current_owner_pkg, justification } => {
            let mut s = String::new();
            s.push_str("    <characteristic type=\"DeviceAdminMgr\">\n");
            s.push_str(&parm("Action", "Deenroll"));
            s.push_str(&parm("OwnerPackage", current_owner_pkg));
            s.push_str(&parm("Justification", justification));
            s.push_str("    </characteristic>\n");
            s
        }
        Stage::SetProperty { key, value } => {
            let mut s = String::new();
            s.push_str("    <characteristic type=\"PropertyMgr\">\n");
            s.push_str(&parm(key, value));
            s.push_str("    </characteristic>\n");
            s
        }
        Stage::OtaUpdate { url, sha256 } => {
            let mut s = String::new();
            s.push_str("    <characteristic type=\"OsUpdateMgr\">\n");
            s.push_str(&parm("PackageURL", url));
            s.push_str(&parm("SHA256", sha256));
            s.push_str("    </characteristic>\n");
            s
        }
    }
}

fn parm(k: &str, v: &str) -> String {
    format!("      <parm name=\"{}\" value=\"{}\"/>\n", xml_escape(k), xml_escape(v))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_profile_xml_well_formed() {
        let p = StageNowProfile {
            name: "Test".into(), author: "Chimera".into(),
            version: "1.0".into(), stages: vec![],
        };
        let x = p.to_xml();
        assert!(x.starts_with("<?xml"));
        assert!(x.contains("name=\"Test\""));
        assert!(x.contains("</wap-provisioningdoc>"));
    }

    #[test]
    fn wifi_psk_emits_psk_parm() {
        let p = StageNowProfile {
            name: "wifi".into(), author: "x".into(), version: "1.0".into(),
            stages: vec![Stage::WifiConfig(WifiConfig {
                ssid: "ShopFloor".into(),
                security: WifiSecurity::Wpa2Psk { password: "letmein".into() },
                hidden: false, proxy_host: None, proxy_port: None,
            })],
        };
        let x = p.to_xml();
        assert!(x.contains("SSID"));
        assert!(x.contains("ShopFloor"));
        assert!(x.contains("PSK"));
        assert!(x.contains("letmein"));
    }

    #[test]
    fn mx_toggles_emit_enable_disable() {
        let p = StageNowProfile {
            name: "mx".into(), author: "x".into(), version: "1.0".into(),
            stages: vec![Stage::Mx(MxToggles {
                bluetooth_enabled: Some(false),
                camera_enabled:    Some(true),
                adb_over_usb:      Some(true),
                ..Default::default()
            })],
        };
        let x = p.to_xml();
        assert!(x.contains("Bluetooth"));
        assert!(x.contains("Disable"));
        assert!(x.contains("AdbOverUsb"));
        assert!(x.contains("Enable"));
    }

    #[test]
    fn xml_escape_handles_special_chars() {
        let s = xml_escape("Hello & <World>");
        assert!(s.contains("&amp;"));
        assert!(s.contains("&lt;"));
        assert!(s.contains("&gt;"));
    }

    #[test]
    fn fingerprint_changes_with_content() {
        let mut p = StageNowProfile {
            name: "x".into(), author: "y".into(), version: "1.0".into(), stages: vec![]
        };
        let f1 = p.fingerprint();
        p.name = "different".into();
        let f2 = p.fingerprint();
        assert_ne!(f1, f2);
        assert_eq!(f1.len(), 16);
    }

    #[test]
    fn factory_reset_serialises() {
        let p = StageNowProfile {
            name: "wipe".into(), author: "x".into(), version: "1.0".into(),
            stages: vec![Stage::FactoryReset { preserve_sd_card: true }],
        };
        let x = p.to_xml();
        assert!(x.contains("FactoryReset"));
        assert!(x.contains("PreserveSDCard"));
        assert!(x.contains("true"));
    }
}
