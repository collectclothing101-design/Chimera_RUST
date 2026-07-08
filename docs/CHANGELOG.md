## [v1.3.4] — 2026-03-13  ChimeraTool-Matched GUI Polish

### ✨ GUI — Full ChimeraTool Aesthetic Overhaul

**theme.rs** — Palette: cyan → golden amber (#F5A623, Chimera griffin-logo colour). New constants: BG_DARK/BG_SIDEBAR/BG_CARD/BG_ELEVATED/BG_ACTIVE/BG_HEADER plus full ACCENT family. Custom nav_item(), nav_section_label(), accent_button(), outline_button(), card_frame(), golden_badge(), status_dot(). Complete build_visuals() rewrite with amber selection highlights.

**app.rs** — Layout mirrors ChimeraTool: header_bar (42px) → nav_sidebar (190px) → CentralPanel → log_panel (bottom). All frames use new palette. Calls ChimeraTheme::apply().

**ui/mod.rs** — nav_sidebar() with sections Device/Apple-iOS/Unlock/General, device badge count, compact sidebar device list. render_content_tabs() with golden-underline tab strip. tab_button() custom drawn with amber underline on active. no_device_hint() amber phone icon. route_content() clean panel routing.

**ui/menu.rs** — header_bar renders ⚡ CHIMERA RS brand, File/Tools/Help menus, right-side device status dot + RESEARCH BUILD amber badge.

**ui/device_list.rs** — Sidebar compact amber-highlighted cards + full DeviceInfo card view. Empty-state card. Brand colour dot, state dot, ACTIVE golden badge.

**ui/device_info.rs** — Updated signature (ui, state, device_id, op_tx). Hero card with brand-coloured border. Two-column card_frame grids. Amber spinner progress.

**ui/common.rs** — section_header() with amber left-bar. Full widget suite: status badges, notification boxes (warning/success/error/info/legal), kv_row, code_block, divider, file/dir pickers.

**ui/log_panel.rs** — Console header with amber ▶ icon. [LVL] coloured log entries, sticky-to-bottom scroll.

**ui/about.rs** — Amber ⚡ logo, detail grid, legal disclaimer, amber close button.

**ui/settings_panel.rs** — Card-framed ADB/Paths/Appearance/Behaviour columns. Accent+Outline action buttons.

### Build Stats: LOC 24,979 → 25,996 (+1,017) | Brace-balance PASS | No stale SURFACE/HEADER_BG refs

---

# ChimeraRS Changelog

## v1.3.1 — SHSH Blob Engine + AU Carrier Protocol (2026-03-13)

### New Modules
| Module | File | Lines | Description |
|--------|------|-------|-------------|
| `chimera_apple::shsh` | `shsh.rs` | 1,153 | Full SHSH2 blob management engine |
| `chimera_apple::au_carrier_unlock` | `au_carrier_unlock.rs` | 829 | AU carrier unlock protocol docs |
| `chimera_gui::ui::shsh_panel` | `shsh_panel.rs` | 539 | SHSH Blob Manager GUI panel |

### shsh.rs — Key Types & Functions
- **`Shsh2Blob`** — parsed SHSH2 blob with ECID, APNonce, generator, SEP compatibility
- **`BlobStore`** — local disk cache at `~/Library/Application Support/ChimeraRS/blobs/`
- **`TssClient`** — Apple TSS (gs.apple.com) + request_ticket() + is_build_signed()
- **`IpswMeClient`** — ipsw.me API for signed firmware lookups
- **`ShshHostClient`** — shsh.host/TSSSaver archive API with fetch_all()
- **`NonceGenerator`** — APNonce generator seed validation and nonce derivation
- **`DowngradeCompatibilityReport`** — chipset/SEP/Cryptex1 downgrade feasibility matrix
- **`FutureRestoreBuilder`** — builds `futurerestore` CLI commands with all flags
- **`ShshErrorCatalogue`** — 8 common SHSH error messages with diagnose/fix text
- **`nonce_generator_instructions()`** — per-method nonce-setter instructions (misaka, SuccessionRestore, palera1n)

### au_carrier_unlock.rs — Key Types & Protocol Docs
- **`AuCarrier`** — 6 AU carriers: Telstra 50501, Optus 50502, Vodafone AU 50503, TPG 50590, Boost 50019, Woolworths 50505
- **`telstra_protocol`**, **`optus_protocol`**, **`vodafone_au_protocol`** — per-carrier HTTP unlock request flow
- **`AuIphoneUnlockWizard::guide()`** — IMEI-validated step-by-step unlock guide
- **`validate_imei()`** — Luhn algorithm IMEI validation
- **`AuUnlockRequest`** + **`UnlockRequestStatus`** — request lifecycle tracking

### restore.rs — Full SHSH Verification Pipeline
- **`IpswRestoreOptions`** now includes: `ecid`, `board_config`, `use_latest_sep`, `use_latest_baseband`, `nonce_generator`
- **`verify_shsh_tss_live()`** — contacts Apple TSS, saves ticket via BlobStore
- **`verify_shsh_local()`** — validates saved .shsh2 blob (ECID/model/nonce/build match)
- **`build_futurerestore_command()`** — generates complete futurerestore CLI string
- Full 9-step restore sequence with SEP/baseband/erase/nonce handling

### operations.rs — New SHSH Operations
- `save_shsh_blob(ecid, board_config, progress)` — TSS ticket request + local save
- `save_all_shsh_from_host(ecid, progress)` — bulk blob save from shsh.host
- `list_cached_shsh_blobs(ecid)` — enumerate local BlobStore
- `get_downgrade_report(target_ios, ecid)` — compatibility report
- `build_futurerestore_cmd(ipsw, blob, sep, baseband, nonce)` — CLI command builder
- `run_au_unlock_wizard(mccmnc, carrier_name)` — AU carrier unlock guide
- `validate_imei(imei)` — static IMEI Luhn check

### state.rs — New Fields
- `shsh_ecid_input`, `shsh_model_input`, `shsh_build_input` — SHSH panel inputs
- `shsh_blob_path`, `shsh_nonce_gen` — restore options
- `shsh_use_latest_sep`, `shsh_use_latest_baseband` — FutureRestore flags
- `shsh_report`, `shsh_futurerestore_cmd`, `shsh_saved_blobs` — output fields
- `shsh_active_tab: ShshTab` — SHSH panel tab state
- **`ShshTab` enum**: SaveBlobs | LocalBlobs | DowngradeReport | FutureRestore | ErrorCatalogue
- `ActiveTab::ShshManager` — new 12th GUI tab

### GUI — SHSH Blob Manager Panel (12th tab)
1. **Save Blobs** — Request APTicket from Apple TSS or shsh.host
2. **Local Blobs** — Browse `~/Library/Application Support/ChimeraRS/blobs/`
3. **Downgrade Report** — SEP/Cryptex1 compatibility matrix
4. **FutureRestore** — Build futurerestore CLI command with all flags
5. **Error Catalogue** — 8 error messages with cause + fix

### lib.rs Re-exports Added
```rust
pub use shsh::{Shsh2Blob, BlobStore, TssClient, IpswMeClient,
               DowngradeCompatibilityReport, FutureRestoreBuilder,
               NonceGenerator, nonce_generator_instructions, ShshErrorCatalogue};
pub use au_carrier_unlock::{AuCarrier, AU_CARRIERS, AuUnlockRequest, UnlockRequestStatus,
                             AuIphoneUnlockWizard, validate_imei, is_apple_imei, ...};
```

---

## v1.3.0 — macOS Port + AU Network Unlock + API Infrastructure (2026-03-12)

### Windows → macOS Port
- Replaced all Win32/winapi/registry/APPDATA references with POSIX/macOS equivalents
- ADB keys: `~/.android/`; App data: `~/Library/Application Support/ChimeraRS/`
- `rust-toolchain.toml` (nightly + x86_64-apple-darwin), `.cargo/config.toml`, `build-macos.sh`

### New Crates
- **`chimera-api`** (12 source files): auth, client, device_api, firmware_api, secure_api,
  upload_api, pics_api, portcheck, mock_server, open_alternatives, endpoints

### Apple Features
- iPhone 16/16e/17 series added to device DB and network_unlock.rs AU table
- iPad Air 16/17 (M3/M4), iPad Pro M4, iPad mini 7, iPad 11th gen
- 34 Apple devices in chimera-devices DB

### GUI Additions (v1.3.0)
- 🌐 API Tools tab (11th tab) — 20 endpoints, mock mode, subdomain map
- 🇦🇺 AU Network Unlock panel — 6 AU carriers, MCC 505 protocol
- 🍎 Apple panel — iCloud bypass, passcode, recovery, flash, AU unlock

---

## v1.2.0 — Initial Release

- 21 crates, 129 source files, ~15,685 LOC
- Samsung, Xiaomi, Motorola, LG, Sony, Nokia, OPPO, Nothing support
- chimera-adb (full ADB protocol), chimera-fastboot, chimera-edl
- chimera-firmware (IPSW/OTA metadata), chimera-devices DB
- 10-tab GUI: Info/Operations/Firmware/Utilities/Diagnostics/History/Settings/Apple/AU/Log

## [1.3.3] — 2026-03-13

### Added — iCloud Endpoint Catalog (`icloud_endpoints.rs`, 2 286 LOC)
- New `chimera-apple` module: `icloud_endpoints.rs`
  - `ICloudEndpoint` struct: fqdn, ipv4[], ipv6[], role, description, requires_mtls, is_china, probe_confirmed
  - `ICloudEndpointRole` enum: 23 variants (TssSigning, FindMyIphone, FindMyFriends, ActivationGateway,
    Gateway, CloudKit, MobileBackup, KeyValueService, EscrowProxy, CalDAV, CardDAV, Drive,
    IWork, Mail, Dns, PrivateRelay, Metrics, Monitor, Connectivity, Beta, Education, Attestation,
    Developer, WebApp, Other)
  - **198 endpoint records** catalogued
  - **27 probe-confirmed** with live IPs (curl probes 2026-03-13)
  - Live IPs captured from active probes and passive DNS:
    - `background.gateway.icloud.com` → 17.248.219.23/66/39/8 (HTTP 301→HTTPS confirmed)
    - `ckhttpapi.icloud.com` → 17.248.219.15/23/66/8 (HTTP 301→HTTPS confirmed)
    - `beta.icloud.com` → 23.11.166.5 (HTTP 301→HTTPS confirmed)
    - `imap.mail.icloud.com` → 17.56.136.196
    - `mailws.icloud.com` → 17.172.192.54
    - `mr-e3sh.icloud.com` → 17.178.103.11 (SHSH relay)
    - `icloud4-hubble.icloud.com` → 17.177.80.31
    - `ic1-wopi.icloud.com`, `ic4-wopi.icloud.com` → 17.248.128.46
    - `antonws.icloud.com` → 17.143.188.8
    - 11 monitorm nodes (17.110.70.x / 17.110.78.x / 17.110.86.x / 17.111.166.x / 17.111.174.x)
    - 6 connectivity nodes (17.178.100.x / 17.178.106.x / 17.110.240.x / 17.110.242.x / 17.110.244.x / 17.110.246.x)
  - Apple IPv4 blocks: 17.110/16, 17.111/16, 17.143/16, 17.172/16, 17.177/16, 17.178/16,
    17.248/16, 17.56.136/24, 23.11.166/24
  - Apple IPv6 blocks: 2403:300:a50:180::/64, 2600:1415:6c00::/48
  - Lookup helpers: `endpoints_by_role`, `endpoints_with_ips`, `probe_confirmed_endpoints`,
    `find_by_fqdn`, `china_endpoints`, `restore_relevant_endpoints`,
    `au_unlock_relevant_endpoints`, `mtls_endpoints`, `endpoint_summary`
  - URL builders: `activation_status_url`, `escrow_proxy_url`, `mcc_unlock_status_url`,
    `gateway_australia_url`, `https_url`, `find_my_device_url`

### Modified — `activation.rs` (+21 LOC)
- Hardcoded `fmipmobile.icloud.com` URL replaced with `activation_status_url()` helper
- Added `check_escrow_key_online(ecid)` — escrowproxy.icloud.com stub
- Added `check_mcc_unlock_status(imei)` — mccgateway.icloud.com stub (AU carrier unlock)

### Modified — `bypass.rs` (+24 LOC)
- Added `DNS_BYPASS_SERVERS` const: 3 community DNS bypass server entries
- Added `activation_gateway_ips()` — returns confirmed background.gateway IPs
- Added `cloudkit_api_ips()` — returns confirmed ckhttpapi IPs for pre-restore checks

### Modified — `operations.rs` (+83 LOC → 547 lines)
- Added `check_escrow_key(progress)` — wraps escrow_proxy_url, logs to diagnostics
- Added `check_mcc_carrier_unlock(progress)` — wraps mcc_unlock_status_url
- Added `check_activation_lock_online(progress)` — wraps fmipmobile activation status check
- Added `icloud_endpoint_summary()` — renders restore + AU-unlock endpoint tables for diagnostics panel

### Modified — `lib.rs`
- Added `pub mod icloud_endpoints`
- Re-exports: `ICloudEndpoint`, `ICloudEndpointRole`, `ICLOUD_ENDPOINTS`,
  all lookup helpers, URL builders, `APPLE_ICLOUD_IPV4_BLOCKS`

### Modified — `api_panel.rs` (+239 LOC → 693 lines)
- New collapsible section: **🍎 iCloud Endpoint Map — Probe-Confirmed & Restore-Relevant**
  - 4-column grid: FQDN · IPv4 · Role · Description
  - 40 entries: all probe-confirmed entries + all restore/AU-relevant endpoints
  - IP block reference footer: 9 IPv4 blocks + 2 IPv6 blocks
  - Server fingerprint note: AppleHttpServer/a3fb6e96e80a

### Build Stats
| Metric | v1.3.2 | v1.3.3 | Δ |
|--------|--------|--------|---|
| RS source files | 146 | 147 | +1 |
| Total LOC | 22 408 | 24 979 | +2 571 |
| chimera-apple modules | 14 | 15 | +1 |
| iCloud endpoints catalogued | 0 | 198 | +198 |
| Probe-confirmed endpoints | 0 | 27 | +27 |
| All brace-balance checks | PASS | PASS | — |
