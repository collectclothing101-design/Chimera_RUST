//
//  AppDelegate.swift
//  NSApplication lifecycle + menu bar + main window orchestration for Chimera.
//
//  Architecture role:
//      AppDelegate owns the single MainWindowController. The window hosts a
//      WebViewController whose root is a WKWebView loading
//      Resources/chimera-gui.html. ChimeraEngine.shared is initialised early
//      so the JS layer can dispatch the moment the DOM is ready.
//

import Cocoa

@main
final class AppDelegate: NSObject, NSApplicationDelegate {

    private var mainWindowController: MainWindowController?

    // MARK: - Lifecycle

    func applicationDidFinishLaunching(_ notification: Notification) {
        // 1. Initialise the Rust engine before opening any windows so the
        //    bridge handler can answer the JS layer's first dispatch call.
        guard ChimeraEngine.shared.initialise() else {
            presentFatalAlert(message: "Failed to initialise ChimeraRS engine.")
            return
        }
        NSLog("[Chimera] Rust engine ready, version \(ChimeraEngine.shared.version())")

        // 2. Build the main menu before showing any window so Cmd-Q etc work.
        buildMainMenu()

        // 3. Open the main window. The window controller wires the web view
        //    bridge and starts the HTML load.
        let controller = MainWindowController()
        controller.showWindow(self)
        mainWindowController = controller
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    func applicationWillTerminate(_ notification: Notification) {
        NSLog("[Chimera] application terminating")
    }

    // MARK: - Menu construction

    /// Programmatic menu bar — avoids depending on a .xib or .storyboard so
    /// the build pipeline stays Cargo + swiftc.
    private func buildMainMenu() {
        let main = NSMenu()

        // ── Application menu ──
        let appItem = NSMenuItem()
        let appMenu = NSMenu()
        appMenu.addItem(withTitle: "About ChimeraRS",
                        action: #selector(showAbout(_:)),
                        keyEquivalent: "")
        appMenu.addItem(NSMenuItem.separator())
        appMenu.addItem(withTitle: "Preferences…",
                        action: #selector(showPreferences(_:)),
                        keyEquivalent: ",")
        appMenu.addItem(NSMenuItem.separator())
        let hideItem = appMenu.addItem(withTitle: "Hide ChimeraRS",
                                       action: #selector(NSApplication.hide(_:)),
                                       keyEquivalent: "h")
        hideItem.target = NSApp
        appMenu.addItem(withTitle: "Hide Others",
                        action: #selector(NSApplication.hideOtherApplications(_:)),
                        keyEquivalent: "h").keyEquivalentModifierMask = [.command, .option]
        appMenu.addItem(withTitle: "Show All",
                        action: #selector(NSApplication.unhideAllApplications(_:)),
                        keyEquivalent: "")
        appMenu.addItem(NSMenuItem.separator())
        appMenu.addItem(withTitle: "Quit ChimeraRS",
                        action: #selector(NSApplication.terminate(_:)),
                        keyEquivalent: "q")
        appItem.submenu = appMenu
        main.addItem(appItem)

        // ── File menu ──
        let fileItem = NSMenuItem()
        let fileMenu = NSMenu(title: "File")
        fileMenu.addItem(withTitle: "Open Firmware…",
                         action: #selector(openFirmware(_:)),
                         keyEquivalent: "o")
        fileMenu.addItem(withTitle: "Export Log…",
                         action: #selector(exportLog(_:)),
                         keyEquivalent: "e")
        fileMenu.addItem(NSMenuItem.separator())
        fileMenu.addItem(withTitle: "Close Window",
                         action: #selector(NSWindow.performClose(_:)),
                         keyEquivalent: "w")
        fileItem.submenu = fileMenu
        main.addItem(fileItem)

        // ── Edit menu (standard) ──
        let editItem = NSMenuItem()
        let editMenu = NSMenu(title: "Edit")
        editMenu.addItem(withTitle: "Undo",
                         action: Selector(("undo:")),
                         keyEquivalent: "z")
        editMenu.addItem(withTitle: "Redo",
                         action: Selector(("redo:")),
                         keyEquivalent: "Z")
        editMenu.addItem(NSMenuItem.separator())
        editMenu.addItem(withTitle: "Cut",
                         action: #selector(NSText.cut(_:)),
                         keyEquivalent: "x")
        editMenu.addItem(withTitle: "Copy",
                         action: #selector(NSText.copy(_:)),
                         keyEquivalent: "c")
        editMenu.addItem(withTitle: "Paste",
                         action: #selector(NSText.paste(_:)),
                         keyEquivalent: "v")
        editMenu.addItem(withTitle: "Select All",
                         action: #selector(NSText.selectAll(_:)),
                         keyEquivalent: "a")
        editItem.submenu = editMenu
        main.addItem(editItem)

        // ── View menu ──
        let viewItem = NSMenuItem()
        let viewMenu = NSMenu(title: "View")
        viewMenu.addItem(withTitle: "Reload Interface",
                         action: #selector(reloadWebView(_:)),
                         keyEquivalent: "r")
        viewMenu.addItem(withTitle: "Toggle Developer Tools",
                         action: #selector(toggleDeveloperTools(_:)),
                         keyEquivalent: "I")
        viewMenu.addItem(NSMenuItem.separator())
        viewMenu.addItem(withTitle: "Enter Full Screen",
                         action: #selector(NSWindow.toggleFullScreen(_:)),
                         keyEquivalent: "f")
        viewItem.submenu = viewMenu
        main.addItem(viewItem)

        // ── Window menu ──
        let windowItem = NSMenuItem()
        let windowMenu = NSMenu(title: "Window")
        windowMenu.addItem(withTitle: "Minimize",
                           action: #selector(NSWindow.performMiniaturize(_:)),
                           keyEquivalent: "m")
        windowMenu.addItem(withTitle: "Zoom",
                           action: #selector(NSWindow.performZoom(_:)),
                           keyEquivalent: "")
        windowItem.submenu = windowMenu
        main.addItem(windowItem)
        NSApp.windowsMenu = windowMenu

        // ── Help menu ──
        let helpItem = NSMenuItem()
        let helpMenu = NSMenu(title: "Help")
        helpMenu.addItem(withTitle: "ChimeraRS Help",
                         action: #selector(showHelp(_:)),
                         keyEquivalent: "?")
        helpItem.submenu = helpMenu
        main.addItem(helpItem)
        NSApp.helpMenu = helpMenu

        NSApp.mainMenu = main
    }

    // MARK: - Menu actions (route to web view)

    @objc private func showAbout(_ sender: Any?) {
        mainWindowController?.webViewController.callJS("ChimeraUI.showAbout()")
    }

    @objc private func showPreferences(_ sender: Any?) {
        mainWindowController?.webViewController.callJS("ChimeraUI.showPreferences()")
    }

    @objc private func openFirmware(_ sender: Any?) {
        let panel = NSOpenPanel()
        panel.allowedFileTypes = ["ipsw", "zip", "tar", "img"]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories    = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        let escaped = url.path.replacingOccurrences(of: "\\", with: "\\\\")
                              .replacingOccurrences(of: "'", with: "\\'")
        mainWindowController?.webViewController
            .callJS("ChimeraUI.firmwareSelected('\(escaped)')")
    }

    @objc private func exportLog(_ sender: Any?) {
        mainWindowController?.webViewController.callJS("ChimeraUI.exportLog()")
    }

    @objc private func reloadWebView(_ sender: Any?) {
        mainWindowController?.webViewController.reload()
    }

    @objc private func toggleDeveloperTools(_ sender: Any?) {
        mainWindowController?.webViewController.toggleDeveloperTools()
    }

    @objc private func showHelp(_ sender: Any?) {
        if let url = URL(string: "https://chimeratool.com/docs") {
            NSWorkspace.shared.open(url)
        }
    }

    // MARK: - Diagnostics

    private func presentFatalAlert(message: String) {
        let alert = NSAlert()
        alert.messageText = "ChimeraRS could not start"
        alert.informativeText = message
        alert.alertStyle = .critical
        alert.addButton(withTitle: "Quit")
        alert.runModal()
        NSApp.terminate(nil)
    }
}
