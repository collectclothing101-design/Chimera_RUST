/*
 * bridge.js
 * Injected at document-start by WebViewController.installBridge().
 * Exposes window.chimera.dispatch({op: '...', ...}) → Promise<response>.
 *
 *   const r = await window.chimera.dispatch({op: 'ping'});
 *   const v = await window.chimera.dispatch({op: 'version'});
 *
 * Internal protocol: every call assigns a UUID, posts the envelope to the
 * `chimera` script-message handler, and parks a resolver in `pending`.
 * Swift answers via `ChimeraBridge._receive(JSON.stringify(envelope))`.
 */

(function () {
  'use strict';

  if (window.ChimeraBridge) { return; } // idempotent

  const pending = new Map();

  function uuid() {
    if (crypto && crypto.randomUUID) { return crypto.randomUUID(); }
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
      const r = (Math.random() * 16) | 0;
      const v = c === 'x' ? r : (r & 0x3) | 0x8;
      return v.toString(16);
    });
  }

  function dispatch(request) {
    return new Promise(function (resolve, reject) {
      if (!request || typeof request !== 'object' || !request.op) {
        reject(new Error('chimera.dispatch: request must be {op: "..."}'));
        return;
      }
      const id = uuid();
      pending.set(id, { resolve: resolve, reject: reject, at: Date.now() });

      const handler = (window.webkit &&
                       window.webkit.messageHandlers &&
                       window.webkit.messageHandlers.chimera);
      if (!handler) {
        pending.delete(id);
        reject(new Error('chimera bridge: native handler missing'));
        return;
      }
      handler.postMessage({ id: id, request: request });
    });
  }

  // Swift calls this synchronously via evaluateJavaScript with a JSON literal.
  function _receive(payload) {
    let env;
    try { env = (typeof payload === 'string') ? JSON.parse(payload) : payload; }
    catch (e) { console.warn('ChimeraBridge: invalid response JSON', payload); return; }
    if (!env || !env.id) { return; }

    const slot = pending.get(env.id);
    if (!slot) { return; }
    pending.delete(env.id);

    const resp = env.response || {};
    if (resp.status === 'ok')  { slot.resolve(resp.data); return; }
    if (resp.status === 'err') { slot.reject(new Error(resp.message || 'engine error')); return; }
    slot.reject(new Error('ChimeraBridge: malformed response'));
  }

  // ── Public surface ───────────────────────────────────────────────
  window.ChimeraBridge = { _receive: _receive };
  window.chimera = {
    dispatch: dispatch,

    // Sugar wrappers — match the same op names the FFI knows.
    ping:          function ()       { return dispatch({ op: 'ping' }); },
    version:       function ()       { return dispatch({ op: 'version' }); },
    listDevices:   function ()       { return dispatch({ op: 'list_devices' }); },
    drainLogs:     function ()       { return dispatch({ op: 'drain_logs' }); },
    validateImei:  function (imei)   { return dispatch({ op: 'validate_imei', imei: imei }); },
    validateMac:   function (mac)    { return dispatch({ op: 'validate_mac',  mac:  mac  }); },
    validateIpsw:  function (path)   { return dispatch({ op: 'validate_ipsw', path: path }); },
    generateQr:    function (text, size) { return dispatch({ op: 'generate_qr', text: text, size: size || 200 }); },
    hostProbes:    function ()       { return dispatch({ op: 'host_probes' }); },
    listIosDevices:function ()       { return dispatch({ op: 'list_ios_devices' }); },
    iosDeviceInfo: function (udid)   { return dispatch({ op: 'ios_device_info', udid: udid }); },
    iosActivationState: function (udid) { return dispatch({ op: 'ios_activation_state', udid: udid }); },
    iosPair:        function (udid)   { return dispatch({ op: 'ios_pair',             udid: udid }); },
    purpleSniff:    function (udid)   { return dispatch({ op: 'purple_sniff',         udid: udid }); },
    purpleRestore:  function (opts)   { return dispatch(Object.assign({ op: 'purple_restore' }, opts || {})); },
    deviceMode:     function (udid)   { return dispatch({ op: 'device_mode',          udid: udid }); },
    readSysCfg:     function (udid)   { return dispatch({ op: 'read_syscfg',          udid: udid }); },
    readBattery:    function (udid)   { return dispatch({ op: 'read_battery',         udid: udid }); },
    samsungReadCodes: function (udid) { return dispatch({ op: 'samsung_read_codes',   udid: udid }); },
    samsungCscSearch: function (query){ return dispatch({ op: 'samsung_csc_search',   query: query }); },
    samsungCscList:   function ()     { return dispatch({ op: 'samsung_csc_list' }); },
    samsungCscChange: function (opts) { return dispatch(Object.assign({ op: 'samsung_csc_change' }, opts || {})); },
    samsungValidateCsc: function (c)  { return dispatch({ op: 'samsung_validate_csc', code: c }); },
    samsungKnoxStatus:function (udid) { return dispatch({ op: 'samsung_knox_status',  udid: udid }); },
    programmerAnalyse:function (path) { return dispatch({ op: 'programmer_analyse',   path: path }); },

    // ─── Auto-Detect Operations ─────────────────────────────────────────
    autoDetect:         function (vid, pid, serial) { return dispatch({ op: 'auto_detect',            vid: vid, pid: pid, serial: serial }); },
    autoDetectAdb:      function ()                  { return dispatch({ op: 'auto_detect_adb' }); },
    getDeviceInfoFull:  function (serial)            { return dispatch({ op: 'get_device_info_full',   serial: serial }); },

    // ─── ChimeraTool Core Features ─────────────────────────────────────
    repairImei:         function (serial, imei1, imei2) { return dispatch({ op: 'repair_imei',         serial: serial, imei1: imei1, imei2: imei2 }); },
    repairMac:          function (serial, mac)          { return dispatch({ op: 'repair_mac',          serial: serial, mac: mac }); },
    factoryReset:       function (serial, brand)        { return dispatch({ op: 'factory_reset',       serial: serial, brand: brand }); },
    enableAdb:          function (serial)               { return dispatch({ op: 'enable_adb',          serial: serial }); },
    rebootDevice:       function (serial, mode)         { return dispatch({ op: 'reboot_device',       serial: serial, mode: mode }); },
    removeScreenLock:   function (serial, brand)        { return dispatch({ op: 'remove_screen_lock',  serial: serial, brand: brand }); },
    updateFirmware:     function (serial, path)         { return dispatch({ op: 'update_firmware',     serial: serial, firmware_path: path }); },

    // ─── Samsung Operations ────────────────────────────────────────────
    samsungGetInfo:     function (serial) { return dispatch({ op: 'samsung_get_info',                serial: serial }); },
    samsungResetFrp:    function (serial) { return dispatch({ op: 'samsung_reset_frp',               serial: serial }); },
    samsungNetworkFactoryReset: function (serial) { return dispatch({ op: 'samsung_network_factory_reset', serial: serial }); },
    samsungResetScreenlock: function (serial) { return dispatch({ op: 'samsung_reset_screenlock',     serial: serial }); },
    samsungRemoveMdm:   function (serial) { return dispatch({ op: 'samsung_remove_mdm',              serial: serial }); },
    samsungRemoveKnoxGuard: function (serial) { return dispatch({ op: 'samsung_remove_knox_guard',   serial: serial }); },
    samsungRepairEfs:   function (serial, path) { return dispatch({ op: 'samsung_repair_efs',        serial: serial, golden_efs_path: path }); },
    samsungStoreBackup: function (serial, path) { return dispatch({ op: 'samsung_store_backup',      serial: serial, output_path: path }); },
    samsungRestoreBackup: function (serial, path) { return dispatch({ op: 'samsung_restore_backup',  serial: serial, backup_path: path }); },
    samsungRemoveLostMode: function (serial) { return dispatch({ op: 'samsung_remove_lost_mode',     serial: serial }); },
    samsungRemoveWarnings: function (serial) { return dispatch({ op: 'samsung_remove_warnings',     serial: serial }); },
    samsungCarrierRelock: function (serial, carriers) { return dispatch({ op: 'samsung_carrier_relock', serial: serial, carriers: carriers }); },
    samsungRemoveDemo:  function (serial) { return dispatch({ op: 'samsung_remove_demo',             serial: serial }); },
    samsungResetReactivationLock: function (serial) { return dispatch({ op: 'samsung_reset_reactivation_lock', serial: serial }); },
    samsungRoot:        function (serial) { return dispatch({ op: 'samsung_root',                    serial: serial }); },

    // ─── Xiaomi Operations ─────────────────────────────────────────────
    xiaomiGetInfo:      function (serial) { return dispatch({ op: 'xiaomi_get_info',                 serial: serial }); },
    xiaomiRemoveFrp:    function (serial) { return dispatch({ op: 'xiaomi_remove_frp',               serial: serial }); },
    xiaomiFactoryReset: function (serial) { return dispatch({ op: 'xiaomi_factory_reset',            serial: serial }); },
    xiaomiNetworkFactoryReset: function (serial) { return dispatch({ op: 'xiaomi_network_factory_reset', serial: serial }); },
    xiaomiRepairImei:   function (serial, imei1, imei2) { return dispatch({ op: 'xiaomi_repair_imei', serial: serial, imei1: imei1, imei2: imei2 }); },
    xiaomiStoreBackup:  function (serial, path) { return dispatch({ op: 'xiaomi_store_backup',       serial: serial, output_path: path }); },
    xiaomiRestoreBackup: function (serial, path) { return dispatch({ op: 'xiaomi_restore_backup',   serial: serial, backup_path: path }); },

    // ─── Huawei Operations ─────────────────────────────────────────────
    huaweiGetInfo:      function (serial) { return dispatch({ op: 'huawei_get_info',                 serial: serial }); },
    huaweiRemoveFrp:    function (serial) { return dispatch({ op: 'huawei_remove_frp',               serial: serial }); },
    huaweiDisableId:    function (serial) { return dispatch({ op: 'huawei_disable_id',               serial: serial }); },
    huaweiFactoryReset: function (serial) { return dispatch({ op: 'huawei_factory_reset',            serial: serial }); },
    huaweiRepairImei:   function (serial, imei1, imei2) { return dispatch({ op: 'huawei_repair_imei', serial: serial, imei1: imei1, imei2: imei2 }); },
    huaweiRemoveDemo:   function (serial) { return dispatch({ op: 'huawei_remove_demo',              serial: serial }); },
    huaweiStoreBackup:  function (serial, path) { return dispatch({ op: 'huawei_store_backup',       serial: serial, output_path: path }); },

    // ─── EDL Operations ────────────────────────────────────────────────
    edlRemoveFrp:       function (sector, lun) { return dispatch({ op: 'edl_remove_frp',            frp_sector: sector, lun: lun }); },
    edlUpdateFirmware:  function (dir)  { return dispatch({ op: 'edl_update_firmware',              firmware_dir: dir }); },
    edlRepairImei:      function (imei1, imei2) { return dispatch({ op: 'edl_repair_imei',          imei1: imei1, imei2: imei2 }); },
    edlStoreBackup:     function (path) { return dispatch({ op: 'edl_store_backup',                 output_path: path }); },

    // ─── Fastboot Operations ───────────────────────────────────────────
    fastbootUnlock:     function ()     { return dispatch({ op: 'fastboot_unlock' }); },
    fastbootLock:       function ()     { return dispatch({ op: 'fastboot_lock' }); },
    fastbootInfo:       function ()     { return dispatch({ op: 'fastboot_info' }); },
    fastbootFlash:      function (partition, path) { return dispatch({ op: 'fastboot_flash',        partition: partition, image_path: path }); },
    fastbootErase:      function (partition) { return dispatch({ op: 'fastboot_erase',              partition: partition }); },
    fastbootReboot:     function (mode) { return dispatch({ op: 'fastboot_reboot',                  mode: mode }); },

    // ─── Network Operations ────────────────────────────────────────────
    readCodes:          function (serial, brand) { return dispatch({ op: 'read_codes',              serial: serial, brand: brand }); },
    networkFactoryReset:function (serial, brand) { return dispatch({ op: 'network_factory_reset',   serial: serial, brand: brand }); },
    patchCertificate:   function (serial, brand) { return dispatch({ op: 'patch_certificate',       serial: serial, brand: brand }); },
    readCertificate:    function (serial, brand) { return dispatch({ op: 'read_certificate',        serial: serial, brand: brand }); },
    writeCertificate:   function (serial, certPath, brand) { return dispatch({ op: 'write_certificate', serial: serial, cert_path: certPath, brand: brand }); },
    unlockBootloader:   function (serial, brand) { return dispatch({ op: 'unlock_bootloader',       serial: serial, brand: brand }); },
    relockBootloader:   function (serial, brand) { return dispatch({ op: 'relock_bootloader',       serial: serial, brand: brand }); },

    // ─── Missing ChimeraTool Operations ────────────────────────────────
    readSpMsl:          function (serial, brand) { return dispatch({ op: 'read_spc_msl',            serial: serial, brand: brand }); },
    resetModemNck:      function (serial, brand) { return dispatch({ op: 'reset_modem_nck',         serial: serial, brand: brand }); },
    setSimCount:        function (serial, count) { return dispatch({ op: 'set_sim_count',           serial: serial, count: count }); },
    backupRpmb:         function (serial, path)  { return dispatch({ op: 'backup_rpmb',             serial: serial, output_path: path }); },
    restoreRpmb:        function (serial, path)  { return dispatch({ op: 'restore_rpmb',            serial: serial, backup_path: path }); },
    networkBackupRestore: function (serial, path, brand) { return dispatch({ op: 'network_backup_restore', serial: serial, output_path: path, brand: brand }); },
    saveModemCalibration: function (serial, path) { return dispatch({ op: 'save_modem_calibration',  serial: serial, output_path: path }); },
    removeBlackberryProtect: function (serial) { return dispatch({ op: 'remove_blackberry_protect',  serial: serial }); },
    removeCommonCriteria: function (serial)   { return dispatch({ op: 'remove_common_criteria',     serial: serial }); },
    removeFmm:          function (serial)      { return dispatch({ op: 'remove_fmm',                 serial: serial }); },
    removeRmm:          function (serial)      { return dispatch({ op: 'remove_rmm',                 serial: serial }); },
    removePleaseCallMeLock: function (serial)  { return dispatch({ op: 'remove_please_call_me_lock', serial: serial }); },
    removeAntiRollbackLock: function (serial)  { return dispatch({ op: 'remove_anti_rollback_lock',  serial: serial }); },
    repairRecovery:     function (serial, path) { return dispatch({ op: 'repair_recovery',           serial: serial, image_path: path }); },
    repairSerial:       function (serial, newSn) { return dispatch({ op: 'repair_serial',            serial: serial, new_serial: newSn }); },
    repairMeid:         function (serial, meid) { return dispatch({ op: 'repair_meid',               serial: serial, meid: meid }); },
    resetBatteryStatus: function (serial)       { return dispatch({ op: 'reset_battery_status',      serial: serial }); },
    resetCamera:        function (serial)       { return dispatch({ op: 'reset_camera',              serial: serial }); },
    resetLcd:           function (serial)       { return dispatch({ op: 'reset_lcd',                 serial: serial }); },
    resetLifetimer:     function (serial)       { return dispatch({ op: 'reset_lifetimer',           serial: serial }); },
    setBatterySerial:   function (serial, bs)   { return dispatch({ op: 'set_battery_serial',        serial: serial, battery_serial: bs }); },
    setKeyboard:        function (serial, layout) { return dispatch({ op: 'set_keyboard',            serial: serial, layout: layout }); },
    setKnoxGuardState:  function (serial, state) { return dispatch({ op: 'set_knox_guard_state',     serial: serial, state: state }); },
    setVendorId:        function (serial, vid)  { return dispatch({ op: 'set_vendor_id',             serial: serial, vendor_id: vid }); },
    enableDiagMode:     function (serial)       { return dispatch({ op: 'enable_diag_mode',          serial: serial }); },
    enterFactoryMode:   function (serial)       { return dispatch({ op: 'enter_factory_mode',        serial: serial }); },
    exitFactoryMode:    function (serial)       { return dispatch({ op: 'exit_factory_mode',         serial: serial }); },
    loadFactoryFastboot: function (serial)      { return dispatch({ op: 'load_factory_fastboot',     serial: serial }); },
    switchToDload:      function (serial)       { return dispatch({ op: 'switch_to_dload',           serial: serial }); },
    switchToEub:        function (serial)       { return dispatch({ op: 'switch_to_eub',             serial: serial }); },
    firmwareCompatibility: function (serial, path) { return dispatch({ op: 'firmware_compatibility',  serial: serial, firmware_path: path }); },
    warrantyCheck:      function (serial)       { return dispatch({ op: 'warranty_check',            serial: serial }); },
    recoverImei:        function (serial)       { return dispatch({ op: 'recover_imei',              serial: serial }); },
    removeMdmGeneric:   function (serial, brand) { return dispatch({ op: 'remove_mdm_generic',       serial: serial, brand: brand }); },
    nuke:               function (serial, brand) { return dispatch({ op: 'nuke',                     serial: serial, brand: brand }); },
    refurbish:          function (serial, brand) { return dispatch({ op: 'refurbish',                serial: serial, brand: brand }); },
    fixDload:           function (serial)       { return dispatch({ op: 'fix_dload',                 serial: serial }); },
    fixBadSectors:      function (serial, part) { return dispatch({ op: 'fix_bad_sectors',           serial: serial, partition: part }); },
    fixChipDamaged:     function (serial)       { return dispatch({ op: 'fix_chip_damaged',          serial: serial }); },
    removeDeviceLock:   function (serial, brand) { return dispatch({ op: 'remove_device_lock',       serial: serial, brand: brand }); },
    advancedUpdateFirmware: function (serial, path, erase) { return dispatch({ op: 'advanced_update_firmware', serial: serial, firmware_path: path, erase: erase }); },
    modelVendorChange:  function (serial, model, vendor, country) { return dispatch({ op: 'model_vendor_change', serial: serial, model: model, vendor: vendor, country: country }); },
    convertToDualSim:   function (serial)       { return dispatch({ op: 'convert_to_dual_sim',      serial: serial }); },
    modemRepair:        function (serial)       { return dispatch({ op: 'modem_repair',              serial: serial }); },
    rootGeneric:        function (serial, brand) { return dispatch({ op: 'root_generic',             serial: serial, brand: brand }); },
    unrootGeneric:      function (serial)       { return dispatch({ op: 'unroot_generic',            serial: serial }); },

    // ─── Zebra TC52 / TC53 ──────────────────────────────────────────
    zebraEnumerate:       function (target) { return dispatch({ op: 'zebra_enumerate',       target: target }); },
    zebraDetectEmm:       function (target) { return dispatch({ op: 'zebra_detect_emm',      target: target }); },
    zebraRxLoggerStart:   function (target) { return dispatch({ op: 'zebra_rxlogger_start',  target: target }); },
    zebraRxLoggerStop:    function (target) { return dispatch({ op: 'zebra_rxlogger_stop',   target: target }); },
    zebraRxLoggerSnapshot:function (target) { return dispatch({ op: 'zebra_rxlogger_snapshot',target: target }); },
    zebraPartitionMap:    function (target) { return dispatch({ op: 'zebra_partition_map',   target: target }); },
    zebraValidatePackage: function (path)   { return dispatch({ op: 'zebra_validate_package',path: path }); },

    // ─── PTT Pro fleet provisioning ─────────────────────────────────
    pttproMockStart:      function ()       { return dispatch({ op: 'pttpro_mock_start' }); },
    pttproMockStop:       function ()       { return dispatch({ op: 'pttpro_mock_stop' }); },
    pttproListUsers:      function (env)    { return dispatch(Object.assign({ op: 'pttpro_list_users' }, env || {})); },
    pttproCreateUser:     function (opts)   { return dispatch(Object.assign({ op: 'pttpro_create_user' }, opts || {})); },
    pttproEnrollDevice:   function (opts)   { return dispatch(Object.assign({ op: 'pttpro_enroll_device' }, opts || {})); },
    pttproGenerateCode:   function (opts)   { return dispatch(Object.assign({ op: 'pttpro_generate_code' }, opts || {})); },
    pttproBulkCsv:        function (opts)   { return dispatch(Object.assign({ op: 'pttpro_bulk_csv' }, opts || {})); },
  };

  // ── ChimeraUI namespace ──────────────────────────────────────────
  // Hooks the native menu bar calls into. Pages override these to react
  // to Open Firmware… / Export Log / etc.
  window.ChimeraUI = window.ChimeraUI || {
    showAbout:        function () { document.getElementById('about-modal')?.classList.add('open'); },
    showPreferences:  function () { window.location.hash = '#settings'; },
    firmwareSelected: function (path) {
      const evt = new CustomEvent('chimera:firmware-selected', { detail: { path: path } });
      window.dispatchEvent(evt);
    },
    exportLog: function () {
      const evt = new CustomEvent('chimera:export-log');
      window.dispatchEvent(evt);
    },
  };

  // ── Ready signal ─────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', function () {
    const evt = new CustomEvent('chimera:ready');
    window.dispatchEvent(evt);
    // Optional: ping the engine so the dashboard can show "engine: ready"
    dispatch({ op: 'version' })
      .then(function (v) {
        console.log('[Chimera] engine ready:', v);
        const evt2 = new CustomEvent('chimera:engine-version', { detail: v });
        window.dispatchEvent(evt2);
      })
      .catch(function (e) {
        console.warn('[Chimera] engine ping failed:', e);
      });
  });
})();
