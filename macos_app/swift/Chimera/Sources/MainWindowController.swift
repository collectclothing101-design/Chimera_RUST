//
//  MainWindowController.swift
//  Owns the main NSWindow. Configures titlebar, autosave, minimum size,
//  and hosts the WebViewController.
//

import Cocoa

final class MainWindowController: NSWindowController, NSWindowDelegate {

    let webViewController = WebViewController()

    convenience init() {
        let contentRect = NSRect(x: 0, y: 0, width: 1440, height: 900)
        let style: NSWindow.StyleMask = [
            .titled, .closable, .miniaturizable, .resizable,
            .fullSizeContentView,
        ]
        let window = NSWindow(contentRect: contentRect,
                              styleMask:   style,
                              backing:     .buffered,
                              defer:       false)
        window.title                  = "ChimeraRS"
        window.titlebarAppearsTransparent = true
        window.titleVisibility        = .visible
        window.isReleasedWhenClosed   = false
        window.minSize                = NSSize(width: 1080, height: 720)
        window.center()
        window.setFrameAutosaveName("ChimeraMainWindow")
        window.backgroundColor        = NSColor(red:  0.07, green: 0.05, blue: 0.04, alpha: 1.0)

        self.init(window: window)
        window.delegate    = self
        window.contentView = webViewController.view

        // Load the HTML page immediately since this is a programmatic window
        // (windowDidLoad only fires for nib/storyboard-based windows).
        webViewController.loadInitialPage()
    }

    // MARK: - NSWindowDelegate

    func windowWillClose(_ notification: Notification) {
        NSLog("[Chimera] main window closing")
    }

    func windowDidBecomeKey(_ notification: Notification) {
        // Re-focus the web view's input handling when the window regains focus.
        webViewController.view.window?.makeFirstResponder(webViewController.view)
    }
}
