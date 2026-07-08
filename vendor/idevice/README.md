# Vendored libimobiledevice Toolchain

This directory ships pre-compiled `libimobiledevice` binaries that Chimera's
iOS workflows shell out to via `chimera-imobile`. The resolution order
(implemented in `chimera_imobile::tool::resolve()`) consults:

1. Per-tool environment-variable override (e.g. `CHIMERA_IDEVICE_ID`)
2. `$PATH` lookup via the `which` crate
3. **This directory** — bundled into `Chimera.app/Contents/Resources/idevice/`
   by `deploy/build_app.sh`
4. Common install prefixes (`/usr/local/bin`, `/opt/homebrew/bin`, `/usr/bin`)

## Sub-directories

```
vendor/idevice/
├── windows/    # Windows 64-bit binaries (libimobiledevice 1.3.0-ish)
├── arm64/      # macOS Apple Silicon (populated by `brew install …` + bottling)
├── x86_64/     # macOS Intel
└── README.md
```

## Windows toolchain provenance

The Windows binaries in `windows/` were originally compiled by GitHub user
*iFred09* in May 2020 and distributed with the `#007-RAMDISK6.6` package.
They include:

- 31 `idevice_*.exe` tools (idevice_id, ideviceinfo, ideviceactivation,
  idevicebackup, idevicebackup2, idevicecrashreport, idevicedate,
  idevicedebug, idevicedebugserverproxy, idevicediagnostics,
  ideviceenterrecovery, ideviceimagemounter, ideviceinfo,
  ideviceinstaller, idevicename, idevicenotificationproxy, idevicepair,
  ideviceprovision, idevicerestore, idevicescreenshot, idevicesetlocation,
  idevicesyslog, inetcat, ios_webkit_debug_proxy, iproxy, irecovery,
  plist_cmp, plist_test, plistutil, usbmuxd, win-plutil)
- 54 supporting DLLs (`libimobiledevice-1.0`, `libusbmuxd-2.0`, `libplist-2.0`,
  `libssh2`, `libssl-1_1`, `libcrypto-1_1`, `libcurl`, `libreadline`, `libxml2`,
  `libusb-1.0`, `libusb0`, `libpthread`, `libiconv`, `libintl-8`, `libffi`,
  `libgmp`, `libgnutls-30`, `libidn-11`, `libnettle-6`, `libtasn1-6`, `libffi-6`,
  `liblzma-5`, `libbz2-1`, `libnghttp2-14`, `libpsl-5`, `libxml2-2`, `libzip-4`,
  `libbrotlicommon`, `libbrotlidec`, `libcharset`, `libgcc_s_dw2-1`, `libhogweed-4`,
  `libidn2-0`, `libp11-kit-0`, `libplist-2.0`, `libplist++-3`, `libreadline6`,
  `librtmp-1`, `libssh2-1`, `libtermcap-0`, `libunistring-2`, `libwinpthread-1`,
  `pcre`, `pthreadVC3`, `vcruntime140`, `zlib1`, etc.)

## macOS toolchain provenance

`arm64/` and `x86_64/` are populated either:

- Manually by the maintainer, or
- Automatically by `build_app.sh` from `brew install libimobiledevice` at
  build time.

Building from source is straightforward — see
[libimobiledevice.org](https://libimobiledevice.org/) for canonical sources.

## Licensing

`libimobiledevice` and the supporting libraries are released under
**LGPL-2.1-or-later** (most components) and **GPL-2.0-or-later** (a few tools).
Verbatim distribution requires accompanying source-code availability; the
canonical sources are published at <https://github.com/libimobiledevice>.

When shipping Chimera with these binaries you are responsible for honouring
the LGPL/GPL terms — the simplest approach is to include the upstream source
tarballs alongside the .app or link to the canonical repository in your
release notes.

Apple's USB driver INFs (`usbaapl64.inf`, `Apple_Mobile_Device_(DFU_Mode).inf`,
`netaapl64.inf`) are Apple-copyrighted and **not** redistributed in this
directory — only the VID/PID lookup table they describe is encoded in
`crates/chimera-core/src/usb.rs`, which is a factual catalogue of public
USB descriptors.
