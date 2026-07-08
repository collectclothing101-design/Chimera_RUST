//
//  WebViewController.swift
//  Hosts the WKWebView that renders Resources/chimera-gui.html.
//  Wires the BridgeMessageHandler so the JS layer can call into Rust.
//

import Cocoa
import WebKit

final class WebViewController: NSViewController {

    // MARK: - Web view

    let webView: WKWebView = {
        let config = WKWebViewConfiguration()

        // Allow file:// → file:// XHR/fetch so the bundled HTML can pull in
        // sibling CSS/JS assets at load time.
        config.preferences.setValue(true, forKey: "allowFileAccessFromFileURLs")
        config.setValue(true, forKey: "allowUniversalAccessFromFileURLs")

        // Web-inspector support — toggled at runtime by the View menu.
        if #available(macOS 13.3, *) {
            config.preferences.isElementFullscreenEnabled = true
        }

        let v = WKWebView(frame: .zero, configuration: config)
        v.allowsBackForwardNavigationGestures = false
        v.allowsMagnification = false
        v.setValue(false, forKey: "drawsBackground")
        return v
    }()

    // MARK: - Bridge handler

    private var bridgeHandler: BridgeMessageHandler?

    // MARK: - Lifecycle

    override func loadView() {
        view = NSView(frame: NSRect(x: 0, y: 0, width: 1440, height: 900))
        view.wantsLayer = true
        view.layer?.backgroundColor = NSColor(red: 0.07, green: 0.05, blue: 0.04, alpha: 1.0).cgColor

        webView.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(webView)
        NSLayoutConstraint.activate([
            webView.topAnchor.constraint(equalTo: view.topAnchor),
            webView.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            webView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            webView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        ])

        installBridge()
    }

    private func installBridge() {
        let handler = BridgeMessageHandler { [weak self] json in
            self?.callJS("ChimeraBridge._receive(\(json))")
        }
        bridgeHandler = handler

        let userContent = webView.configuration.userContentController
        // removeAllScriptMessageHandlers() requires macOS 11+; for 10.14 compat
        // remove the specific handler by name if it was previously registered.
        if #available(macOS 11.0, *) {
            userContent.removeAllScriptMessageHandlers()
        } else {
            userContent.removeScriptMessageHandler(forName: "chimera")
        }
        userContent.add(handler, name: "chimera")

        // Inject the JS shim that exposes window.chimera.dispatch() before
        // any page script runs.
        if let url = Bundle.main.url(forResource: "bridge", withExtension: "js"),
           let js = try? String(contentsOf: url, encoding: .utf8) {
            let script = WKUserScript(source:        js,
                                      injectionTime: .atDocumentStart,
                                      forMainFrameOnly: true)
            userContent.addUserScript(script)
        }
    }

    func loadInitialPage() {
        guard let htmlURL = Bundle.main.url(forResource: "chimera-gui",
                                            withExtension: "html") else {
            loadFallbackPage(reason: "chimera-gui.html missing from Resources/")
            return
        }
        let baseDir = htmlURL.deletingLastPathComponent()
        webView.loadFileURL(htmlURL, allowingReadAccessTo: baseDir)
    }

    private func loadFallbackPage(reason: String) {
        let html = """
        <html><body style='background:#141014;color:#ffe9b6;
          font-family:-apple-system,Helvetica;padding:48px;'>
          <h1>ChimeraRS</h1>
          <p>The HTML interface failed to load.</p>
          <pre>\(reason)</pre>
        </body></html>
        """
        webView.loadHTMLString(html, baseURL: nil)
    }

    // MARK: - JS bridge helpers

    /// Evaluate JavaScript on the web view, swallowing the result. The
    /// caller is responsible for the safety of the expression — any
    /// user-supplied string MUST be escaped before interpolation.
    func callJS(_ expression: String) {
        DispatchQueue.main.async {
            self.webView.evaluateJavaScript(expression) { _, error in
                if let error = error {
                    NSLog("[Chimera] JS error in `\(expression)`: \(error)")
                }
            }
        }
    }

    func reload() {
        loadInitialPage()
    }

    func toggleDeveloperTools() {
        let current = webView.configuration.preferences
            .value(forKey: "developerExtrasEnabled") as? Bool ?? false
        webView.configuration.preferences
            .setValue(!current, forKey: "developerExtrasEnabled")
        callJS("console.log('Developer extras: \(!current)')")
    }
}
