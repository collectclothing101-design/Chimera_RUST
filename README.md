# ChimeraRS v1.4.0

A macOS Universal hybrid application that pairs a Rust engine with a
Swift/WebKit host running an HTML/CSS/JS interface. Targets macOS 10.14
(Mojave) Intel + Apple Silicon.

**Open-source reimplementation of ChimeraTool** — no login, no credits, no restrictions.

## What's New in v1.4.0

- **50+ new operations** — full ChimeraTool feature parity
- **142 total operations** wired through FFI and JS bridge
- **Universal binary** — works on both Intel and Apple Silicon Macs
- **Signed DMG** — ready for distribution
- **Fixed critical issues** — ReadCodes, SHSH nonce, firmware downloader

## Supported Operations (142 total)

### Core Operations (7)
| Operation | Description |
|-----------|-------------|
| `ping` | Health check |
| `version` | Engine version |
| `list_devices` | Enumerate USB/ADB devices |
| `validate_imei` | IMEI Luhn validation |
| `validate_mac` | MAC address validation |
| `validate_ipsw` | IPSW firmware validation |
| `drain_logs` | Return + clear log buffer |

### Samsung Operations (15)
| Operation | Description |
|-----------|-------------|
| `samsung_get_info` | Full device info |
| `samsung_reset_frp` | Clear FRP lock |
| `samsung_network_factory_reset` | Reset network settings |
| `samsung_reset_screenlock` | Remove screen lock |
| `samsung_remove_mdm` | Remove Knox MDM |
| `samsung_remove_knox_guard` | Remove Knox Guard lock |
| `samsung_repair_efs` | Repair EFS partition |
| `samsung_store_backup` | Backup EFS/security |
| `samsung_restore_backup` | Restore EFS/security |
| `samsung_remove_lost_mode` | Remove Find My Mobile |
| `samsung_remove_warnings` | Remove Knox warnings |
| `samsung_carrier_relock` | Configure carrier lock |
| `samsung_remove_demo` | Remove demo mode |
| `samsung_reset_reactivation_lock` | Remove reactivation lock |
| `samsung_root` | Root device |

### Xiaomi Operations (7)
| Operation | Description |
|-----------|-------------|
| `xiaomi_get_info` | Device info |
| `xiaomi_remove_frp` | FRP removal |
| `xiaomi_factory_reset` | Factory reset |
| `xiaomi_network_factory_reset` | Network reset |
| `xiaomi_repair_imei` | IMEI repair |
| `xiaomi_store_backup` | Backup |
| `xiaomi_restore_backup` | Restore |

### Huawei Operations (7)
| Operation | Description |
|-----------|-------------|
| `huawei_get_info` | Device info |
| `huawei_remove_frp` | FRP removal |
| `huawei_disable_id` | Disable Huawei ID |
| `huawei_factory_reset` | Factory reset |
| `huawei_repair_imei` | IMEI repair |
| `huawei_remove_demo` | Remove demo mode |
| `huawei_store_backup` | Backup |

### EDL Operations (4)
| Operation | Description |
|-----------|-------------|
| `edl_remove_frp` | EDL FRP removal |
| `edl_update_firmware` | Flash firmware |
| `edl_repair_imei` | IMEI repair |
| `edl_store_backup` | EFS backup |

### Fastboot Operations (6)
| Operation | Description |
|-----------|-------------|
| `fastboot_unlock` | Unlock bootloader |
| `fastboot_lock` | Lock bootloader |
| `fastboot_info` | Device info |
| `fastboot_flash` | Flash partition |
| `fastboot_erase` | Erase partition |
| `fastboot_reboot` | Reboot device |

### Network Operations (7)
| Operation | Description |
|-----------|-------------|
| `read_codes` | Read network unlock codes |
| `network_factory_reset` | Generic network reset |
| `patch_certificate` | Patch network certificate |
| `read_certificate` | Read security certificate |
| `write_certificate` | Write security certificate |
| `unlock_bootloader` | Generic bootloader unlock |
| `relock_bootloader` | Generic bootloader relock |

### Service Operations (50+)
| Operation | Description |
|-----------|-------------|
| `repair_imei` | Write new IMEI(s) via ADB |
| `repair_mac` | Rewrite Wi-Fi MAC |
| `factory_reset` | Factory reset device |
| `enable_adb` | Enable ADB |
| `reboot_device` | Reboot to mode |
| `remove_screen_lock` | Remove PIN/pattern |
| `update_firmware` | Update via fastboot |
| `nuke` | Factory reset + FRP + all locks |
| `refurbish` | Full refurbish |
| `warranty_check` | Check warranty status |
| `recover_imei` | Recover IMEI from backup |
| `remove_mdm_generic` | Generic MDM removal |
| `enable_diag_mode` | Enable diagnostic port |
| `enter_factory_mode` | Enter factory mode |
| `exit_factory_mode` | Exit factory mode |
| `switch_to_dload` | Switch to download mode |
| `switch_to_eub` | Switch to EUB mode |
| `modem_repair` | Repair modem baseband |
| `root_generic` | Root device |
| `unroot_generic` | Unroot device |
| `fix_dload` | Fix download mode |
| `fix_bad_sectors` | Repair bad sectors |
| `fix_chip_damaged` | Fix "chip damaged" error |
| ... and 28 more service operations |

### iOS Operations (6)
| Operation | Description |
|-----------|-------------|
| `list_ios_devices` | List iOS devices |
| `ios_device_info` | Lockdownd properties |
| `ios_activation_state` | Activation status |
| `ios_pair` | Pair host with device |
| `purple_sniff` | PurpleSNIFF report |
| `purple_restore` | Purple restore |

### Zebra Operations (7)
| Operation | Description |
|-----------|-------------|
| `zebra_enumerate` | Read device properties |
| `zebra_detect_emm` | Detect EMM agent |
| `zebra_rxlogger_start` | Start RxLogger |
| `zebra_rxlogger_stop` | Stop RxLogger |
| `zebra_rxlogger_snapshot` | Snapshot RxLogger |
| `zebra_partition_map` | Read partition map |
| `zebra_validate_package` | Validate firmware package |

### PTT Pro Operations (7)
| Operation | Description |
|-----------|-------------|
| `pttpro_mock_start` | Start mock server |
| `pttpro_mock_stop` | Stop mock server |
| `pttpro_list_users` | List users |
| `pttpro_create_user` | Create user |
| `pttpro_enroll_device` | Enroll device |
| `pttpro_generate_code` | Generate activation code |
| `pttpro_bulk_csv` | Bulk provisioning |

### Utilities (5)
| Operation | Description |
|-----------|-------------|
| `generate_qr` | QR code generator |
| `host_probes` | Check external tools |
| `read_battery` | Battery info |
| `read_syscfg` | SysCfg block readout |
| `device_mode` | Fast mode probe |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Chimera.app                                            │
│  ─────────                                              │
│  ┌──────────────────┐    ┌───────────────────────────┐  │
│  │ Swift / Cocoa    │    │ HTML / CSS / JS           │  │
│  │ AppDelegate      │◄──►│ chimera-gui.html          │  │
│  │ MainWindow       │ JS │   (loaded in WKWebView)   │  │
│  │ WebViewCtrl      │bridg│   bridge.js shim         │  │
│  └────────┬─────────┘    └───────────────────────────┘  │
│           │ FFI                                         │
│           ▼                                             │
│  ┌──────────────────────────────────────────────────┐   │
│  │ Rust engine ( libchimera_ffi.a / .dylib )        │   │
│  │ ─────────────────────────────────────────────    │   │
│  │ chimera-ffi  →  chimera-core                     │   │
│  │              →  chimera-adb / chimera-apple /    │   │
│  │                 chimera-samsung / chimera-mtk /  │   │
│  │                 chimera-edl / … (23 crates)      │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Workspace Layout

| Path | Purpose |
|------|---------|
| `crates/chimera-core/` | Shared types: `DeviceInfo`, `ChimeraError`, `ChimeraEvent`, `Progress`, IMEI/MAC validators |
| `crates/chimera-adb/` | ADB client + IMEI/MAC repair operations |
| `crates/chimera-apple/` | lockdownd, IPSW restore, activation, recovery mode, SHSH |
| `crates/chimera-samsung/` | Samsung: Odin, FRP, EFS, CSC, Knox, EUB |
| `crates/chimera-xiaomi/` | Xiaomi: EDL, ADB, MiAssistant |
| `crates/chimera-huawei/` | Huawei: Factory Fastboot, HarmonyOS |
| `crates/chimera-mtk/` | MediaTek: BROM + DA |
| `crates/chimera-edl/` | Qualcomm EDL (Sahara + Firehose) |
| `crates/chimera-fastboot/` | Fastboot protocol implementation |
| `crates/chimera-ffi/` | C ABI surface, JSON dispatch (142 operations) |
| `crates/chimera-gui/` | egui-native frontend (alternative) |
| `crates/chimera-api/`, `chimera-firmware/`, `chimera-devices/`, `chimera-utils/` | Cross-cutting services |
| `crates/chimera-imobile/`, `chimera-purple/` | iOS tools |
| `crates/chimera-zebra/`, `chimera-pttpro/` | Enterprise tools |
| `macos_app/swift/` | Swift host: AppDelegate, MainWindow, WebView, Bridge |
| `docs/chimera-gui.html` | The HTML UI (3,419 lines, 18 pages) |
| `macos_app/swift/Chimera/Resources/bridge.js` | JS bridge (142 operation wrappers) |
| `deploy/build_rust.sh` | Cross-arch Rust build → lipo |
| `deploy/build_app.sh` | End-to-end .app assembly |
| `deploy/release.sh` | Build + package + publish to GitHub |

## Build

### Quick start (host arch, debug)

```bash
./deploy/build_app.sh --universal --no-sign
open target/debug/Chimera.app
```

### Release universal binary, signed for distribution

```bash
./deploy/build_app.sh --release --universal
# → target/release/Chimera.app
```

### Create DMG for distribution

```bash
./deploy/package_dmg.sh --release
# → ChimeraRS_1.4.0.dmg
```

### Publish to GitHub Releases

```bash
./deploy/release.sh --version 1.4.0
# Creates tag, release, and uploads DMG
```

### Just the Rust workspace

```bash
cargo check --workspace        # 0 errors expected
cargo test  --workspace        # 43+ unit tests
cargo build --release -p chimera-ffi  # libchimera_ffi.{a,dylib}
```

### Just the egui frontend (alternative to Swift host)

```bash
cargo run --release -p chimera-gui
```

## FFI Surface

The Rust engine exposes 4 C functions:

```c
int   chimera_init(void);                          // 0 on success
char *chimera_version(void);                       // free with chimera_string_free
char *chimera_dispatch(const char *request_json);  // free with chimera_string_free
void  chimera_string_free(char *ptr);
```

All non-trivial calls go through `chimera_dispatch` with a JSON envelope:

```json
// request
{"op": "repair_imei", "serial": "ABC123", "imei1": "352099001761481"}

// response
{"status": "ok", "data": {"serial": "ABC123", "imei1": "352099001761481", "status": "success"}}
{"status": "err", "message": "..."}
```

## JS Bridge

The page loaded in WKWebView gets a `window.chimera` global injected at
document-start by `bridge.js`:

```js
// Version check
const v = await window.chimera.version();
//   → {name: "ChimeraRS", version: "1.4.0"}

// IMEI validation
const r = await window.chimera.validateImei("352099001761481");
//   → {input: "352099001761481", valid: true, error: null}

// Samsung FRP reset
await window.chimera.samsungResetFrp("DEVICE_SERIAL");

// Nuke device (factory reset + FRP + all locks)
await window.chimera.nuke("DEVICE_SERIAL", "Samsung");
```

Native menu actions (Open Firmware…, Export Log) dispatch CustomEvents to the
page via `window.ChimeraUI`.

## Testing

```bash
cargo test --workspace                 # Rust unit tests
cargo test -p chimera-ffi              # 43 FFI dispatch tests
cargo test -p chimera-core             # IMEI/MAC/error tests

# XCTest (Mac only)
xcodebuild test -scheme Chimera        # macos_app/swift/ChimeraTests/
```

## Distribution

### Download

**Latest release:** [v1.4.0 DMG](https://github.com/collectclothing101-design/Chimera_RUST/releases/tag/v1.3.13)

### Installation

1. Download the DMG file
2. Mount the DMG
3. Drag `Chimera.app` to Applications
4. Launch from Applications
5. First launch: right-click → Open (Gatekeeper)

### Supported Platforms

- macOS 10.14 Mojave or later
- Intel (x86_64) and Apple Silicon (arm64)

## Status

- ✅ 23 crates, 168+ `.rs` files compile clean
- ✅ 43+ Rust unit tests passing
- ✅ 142 operations wired through FFI
- ✅ Swift host complete (5 source files)
- ✅ JS bridge complete (142 wrappers)
- ✅ HTML GUI complete (3,419 lines, 18 pages)
- ✅ Build pipeline (Rust + Swift + bundle + codesign)
- ✅ Entitlements + sandbox + hardened-runtime ready
- ✅ Universal binary (x86_64 + arm64)
- ✅ DMG packaging
- ✅ GitHub Releases publishing
- ⏳ XCTest harness (skeletons present)
- ⏳ Notarisation

## License

Open source — no login, no credits, no restrictions.
