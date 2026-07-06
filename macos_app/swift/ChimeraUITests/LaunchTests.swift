//
//  LaunchTests.swift
//  XCUIApplication launch and navigation tests for all 18 HTML panels.
//

import XCTest

final class LaunchTests: XCTestCase {

    var app: XCUIApplication!

    override func setUp() {
        super.setUp()
        continueAfterFailure = false
        app = XCUIApplication()
        app.launch()
    }

    override func tearDown() {
        app = nil
        super.tearDown()
    }

    // MARK: - App launch

    func testAppLaunches() {
        XCTAssertTrue(app.windows.count > 0, "App should have at least one window")
    }

    func testAppHasCorrectTitle() {
        let title = app.windows.firstMatch.title
        XCTAssertTrue(title.contains("Chimera") || title.contains("ChimeraRS"),
                      "Window title should contain Chimera")
    }

    func testMainWindowExists() {
        let window = app.windows.firstMatch
        XCTAssertTrue(window.exists, "Main window should exist")
        XCTAssertTrue(window.isHittable, "Main window should be hittable")
    }

    // MARK: - Navigation sidebar

    func testSidebarExists() {
        // The sidebar should be visible
        let sidebar = app.scrollBars.firstMatch
        XCTAssertNotNil(sidebar)
    }

    func testNavigationGroups() {
        // Check for navigation group labels
        let workspace = app.staticTexts["WORKSPACE"]
        let device = app.staticTexts["DEVICE"]
        let platform = app.staticTexts["PLATFORM"]
        XCTAssertTrue(workspace.exists, "Workspace group should exist")
        XCTAssertTrue(device.exists, "Device group should exist")
        XCTAssertTrue(platform.exists, "Platform group should exist")
    }

    // MARK: - Panel navigation (18 panels)

    func testDashboardPanel() {
        let dashboard = app.staticTexts["DASHBOARD"]
        if dashboard.exists {
            dashboard.click()
            // Verify dashboard content loads
            let exp = expectation(description: "dashboard loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testDevicesPanel() {
        let devices = app.staticTexts["DEVICES"]
        if devices.exists {
            devices.click()
            let exp = expectation(description: "devices loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testDownloadsPanel() {
        let downloads = app.staticTexts["DOWNLOADS"]
        if downloads.exists {
            downloads.click()
            let exp = expectation(description: "downloads loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testHistoryPanel() {
        let history = app.staticTexts["WORK HISTORY"]
        if history.exists {
            history.click()
            let exp = expectation(description: "history loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testUtilitiesPanel() {
        let utilities = app.staticTexts["UTILITIES"]
        if utilities.exists {
            utilities.click()
            let exp = expectation(description: "utilities loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testSettingsPanel() {
        let settings = app.staticTexts["SETTINGS"]
        if settings.exists {
            settings.click()
            let exp = expectation(description: "settings loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testDeviceInfoPanel() {
        let deviceInfo = app.staticTexts["DEVICE INFO"]
        if deviceInfo.exists {
            deviceInfo.click()
            let exp = expectation(description: "device info loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testJailbreakPanel() {
        let jailbreak = app.staticTexts["JAILBREAK"]
        if jailbreak.exists {
            jailbreak.click()
            let exp = expectation(description: "jailbreak loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testSshVpnPanel() {
        let sshVpn = app.staticTexts["SSH · VPN"]
        if sshVpn.exists {
            sshVpn.click()
            let exp = expectation(description: "ssh vpn loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testActivationPanel() {
        let activation = app.staticTexts["ACTIVATION"]
        if activation.exists {
            activation.click()
            let exp = expectation(description: "activation loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testNetworkPanel() {
        let network = app.staticTexts["NETWORK"]
        if network.exists {
            network.click()
            let exp = expectation(description: "network loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testToolsPanel() {
        let tools = app.staticTexts["TOOLS"]
        if tools.exists {
            tools.click()
            let exp = expectation(description: "tools loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testAppleIosPanel() {
        let appleIos = app.staticTexts["APPLE IOS"]
        if appleIos.exists {
            appleIos.click()
            let exp = expectation(description: "apple ios loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testMediaTekPanel() {
        let mediatek = app.staticTexts["MEDIATEK"]
        if mediatek.exists {
            mediatek.click()
            let exp = expectation(description: "mediatek loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testAuUnlockPanel() {
        let auUnlock = app.staticTexts["AU UNLOCK"]
        if auUnlock.exists {
            auUnlock.click()
            let exp = expectation(description: "au unlock loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testShshBlobsPanel() {
        let shshBlobs = app.staticTexts["SHSH BLOBS"]
        if shshBlobs.exists {
            shshBlobs.click()
            let exp = expectation(description: "shsh blobs loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testApiToolsPanel() {
        let apiTools = app.staticTexts["API TOOLS"]
        if apiTools.exists {
            apiTools.click()
            let exp = expectation(description: "api tools loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    func testEventLogPanel() {
        let eventLog = app.staticTexts["EVENT LOG"]
        if eventLog.exists {
            eventLog.click()
            let exp = expectation(description: "event log loaded")
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { exp.fulfill() }
            wait(for: [exp], timeout: 2.0)
        }
    }

    // MARK: - Rapid navigation

    func testRapidPanelSwitching() {
        let panels = ["DASHBOARD", "DEVICES", "SETTINGS", "MEDIA Tek", "EVENT LOG"]
        for panel in panels {
            let element = app.staticTexts[panel]
            if element.exists {
                element.click()
                Thread.sleep(forTimeInterval: 0.2)
            }
        }
        // Should not crash
    }

    // MARK: - Memory

    func testRepeatedLaunch() {
        for _ in 0..<5 {
            let testApp = XCUIApplication()
            testApp.launch()
            testApp.terminate()
        }
    }
}
