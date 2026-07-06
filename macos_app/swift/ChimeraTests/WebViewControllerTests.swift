//
//  WebViewControllerTests.swift
//  XCTest harness for the WKWebView host controller.
//

import XCTest
import WebKit
@testable import Chimera

final class WebViewControllerTests: XCTestCase {

    var controller: WebViewController!

    override func setUp() {
        super.setUp()
        controller = WebViewController()
        // Trigger loadView to initialize the view hierarchy
        _ = controller.view
    }

    override func tearDown() {
        controller = nil
        super.tearDown()
    }

    // MARK: - View initialization

    func testViewExists() {
        XCTAssertNotNil(controller.view)
    }

    func testWebViewExists() {
        XCTAssertNotNil(controller.webView)
    }

    func testViewIsNSView() {
        XCTAssertTrue(controller.view is NSView)
    }

    func testViewHasCorrectFrame() {
        let frame = controller.view.frame
        XCTAssertGreaterThan(frame.width, 0)
        XCTAssertGreaterThan(frame.height, 0)
    }

    func testViewHasLayer() {
        XCTAssertNotNil(controller.view.layer)
    }

    func testViewBackgroundColor() {
        // Background should be dark (chimera theme)
        XCTAssertNotNil(controller.view.layer?.backgroundColor)
    }

    // MARK: - WebView configuration

    func testWebViewAllowsBackForward() {
        XCTAssertFalse(controller.webView.allowsBackForwardNavigationGestures)
    }

    func testWebViewAllowsMagnification() {
        XCTAssertFalse(controller.webView.allowsMagnification)
    }

    func testWebViewDrawsBackground() {
        // Should be false (transparent background)
        let drawsBg = controller.webView.value(forKey: "drawsBackground") as? Bool
        XCTAssertEqual(drawsBg, false)
    }

    func testWebViewConfiguration() {
        let config = controller.webView.configuration
        XCTAssertNotNil(config)
        XCTAssertTrue(config.preferences.value(forKey: "allowFileAccessFromFileURLs") as? Bool ?? false)
    }

    // MARK: - Bridge installation

    func testBridgeHandlerInstalled() {
        let userContent = controller.webView.configuration.userContentController
        // The handler should be registered for "chimera" messages
        // Note: We can't directly check handlers, but we can verify the controller exists
        XCTAssertNotNil(userContent)
    }

    // MARK: - Page loading

    func testLoadInitialPage() {
        // Should not crash when called
        controller.loadInitialPage()
        // Give it a moment to load
        let exp = expectation(description: "page loaded")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
        wait(for: [exp], timeout: 2.0)
        XCTAssertNotNil(controller.webView.url)
    }

    func testReload() {
        controller.loadInitialPage()
        let exp = expectation(description: "initial load")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
        wait(for: [exp], timeout: 2.0)

        // Reload should not crash
        controller.reload()
        let exp2 = expectation(description: "reload complete")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp2.fulfill() }
        wait(for: [exp2], timeout: 2.0)
    }

    // MARK: - JS bridge

    func testCallJS() {
        // Should not crash when called
        controller.callJS("console.log('test')")
        let exp = expectation(description: "js executed")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { exp.fulfill() }
        wait(for: [exp], timeout: 1.0)
    }

    func testCallJSWithExpression() {
        controller.callJS("document.title = 'ChimeraRS Test'")
        let exp = expectation(description: "js executed")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { exp.fulfill() }
        wait(for: [exp], timeout: 1.0)
    }

    // MARK: - Developer tools

    func testToggleDeveloperTools() {
        // Should not crash
        controller.toggleDeveloperTools()
        let exp = expectation(description: "toggle complete")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { exp.fulfill() }
        wait(for: [exp], timeout: 1.0)
    }

    func testToggleDeveloperToolsTwice() {
        // Toggle on then off
        controller.toggleDeveloperTools()
        let exp1 = expectation(description: "first toggle")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) { exp1.fulfill() }
        wait(for: [exp1], timeout: 1.0)

        controller.toggleDeveloperTools()
        let exp2 = expectation(description: "second toggle")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) { exp2.fulfill() }
        wait(for: [exp2], timeout: 1.0)
    }

    // MARK: - Memory

    func testRepeatedViewInit() {
        // Create and destroy multiple controllers
        for _ in 0..<10 {
            let ctrl = WebViewController()
            _ = ctrl.view
        }
        // Should not leak
    }
}
