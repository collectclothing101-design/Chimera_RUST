//
//  BridgeMessageHandlerTests.swift
//  XCTest harness for the JS → Swift envelope handler.
//

import XCTest
import WebKit
@testable import Chimera

final class BridgeMessageHandlerTests: XCTestCase {

    private var lastForward: String?
    private var handler: BridgeMessageHandler!

    override func setUp() {
        super.setUp()
        _ = ChimeraEngine.shared.initialise()
        lastForward = nil
        handler = BridgeMessageHandler { [weak self] json in
            self?.lastForward = json
        }
    }

    // MARK: - Envelope shape

    func testValidEnvelopeProducesResponse() {
        let body: [String: Any] = [
            "id":      "test-001",
            "request": ["op": "ping"],
        ]
        let message = FakeScriptMessage(name: "chimera", body: body)
        handler.userContentController(WKUserContentController(), didReceive: message)

        // Bridge dispatches on a background queue; wait briefly.
        let exp = expectation(description: "response received")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) {
            if self.lastForward != nil { exp.fulfill() }
        }
        wait(for: [exp], timeout: 2.0)
        XCTAssertNotNil(lastForward)
        XCTAssertTrue(lastForward?.contains("test-001") == true)
    }

    func testMissingIdIsIgnored() {
        let body: [String: Any] = ["request": ["op": "ping"]] // no id
        let message = FakeScriptMessage(name: "chimera", body: body)
        handler.userContentController(WKUserContentController(), didReceive: message)
        // Wait briefly + assert no forward
        let exp = expectation(description: "no forward")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) { exp.fulfill() }
        wait(for: [exp], timeout: 1.0)
        XCTAssertNil(lastForward)
    }

    func testMalformedOpReturnsErr() {
        let body: [String: Any] = [
            "id":      "test-002",
            "request": ["op": "this-op-does-not-exist"],
        ]
        let message = FakeScriptMessage(name: "chimera", body: body)
        handler.userContentController(WKUserContentController(), didReceive: message)

        let exp = expectation(description: "err returned")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) {
            if self.lastForward?.contains("test-002") == true { exp.fulfill() }
        }
        wait(for: [exp], timeout: 2.0)
        XCTAssertTrue(lastForward?.contains("\"status\":\"err\"") == true)
    }

    // MARK: - Typed requests via bridge

    func testPingViaBridge() {
        let body: [String: Any] = [
            "id":      "bridge-ping",
            "request": ["op": "ping"],
        ]
        let message = FakeScriptMessage(name: "chimera", body: body)
        handler.userContentController(WKUserContentController(), didReceive: message)

        let exp = expectation(description: "ping response")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) {
            if self.lastForward?.contains("bridge-ping") == true { exp.fulfill() }
        }
        wait(for: [exp], timeout: 2.0)
        XCTAssertTrue(lastForward?.contains("\"status\":\"ok\"") == true)
    }

    func testVersionViaBridge() {
        let body: [String: Any] = [
            "id":      "bridge-version",
            "request": ["op": "version"],
        ]
        let message = FakeScriptMessage(name: "chimera", body: body)
        handler.userContentController(WKUserContentController(), didReceive: message)

        let exp = expectation(description: "version response")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) {
            if self.lastForward?.contains("bridge-version") == true { exp.fulfill() }
        }
        wait(for: [exp], timeout: 2.0)
        XCTAssertTrue(lastForward?.contains("\"status\":\"ok\"") == true)
    }

    func testValidateImeiViaBridge() {
        let body: [String: Any] = [
            "id":      "bridge-imei",
            "request": ["op": "validate_imei", "imei": "352099001761481"],
        ]
        let message = FakeScriptMessage(name: "chimera", body: body)
        handler.userContentController(WKUserContentController(), didReceive: message)

        let exp = expectation(description: "imei response")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) {
            if self.lastForward?.contains("bridge-imei") == true { exp.fulfill() }
        }
        wait(for: [exp], timeout: 2.0)
        XCTAssertTrue(lastForward?.contains("\"status\":\"ok\"") == true)
    }

    func testValidateMacViaBridge() {
        let body: [String: Any] = [
            "id":      "bridge-mac",
            "request": ["op": "validate_mac", "mac": "AA:BB:CC:DD:EE:FF"],
        ]
        let message = FakeScriptMessage(name: "chimera", body: body)
        handler.userContentController(WKUserContentController(), didReceive: message)

        let exp = expectation(description: "mac response")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) {
            if self.lastForward?.contains("bridge-mac") == true { exp.fulfill() }
        }
        wait(for: [exp], timeout: 2.0)
        XCTAssertTrue(lastForward?.contains("\"status\":\"ok\"") == true)
    }

    // MARK: - Error handling

    func testMissingRequestIsIgnored() {
        let body: [String: Any] = ["id": "test-no-req"] // no request key
        let message = FakeScriptMessage(name: "chimera", body: body)
        handler.userContentController(WKUserContentController(), didReceive: message)

        let exp = expectation(description: "no forward")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) { exp.fulfill() }
        wait(for: [exp], timeout: 1.0)
        XCTAssertNil(lastForward)
    }

    func testNonDictBodyIsIgnored() {
        let message = FakeScriptMessage(name: "chimera", body: "not a dict")
        handler.userContentController(WKUserContentController(), didReceive: message)

        let exp = expectation(description: "no forward")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) { exp.fulfill() }
        wait(for: [exp], timeout: 1.0)
        XCTAssertNil(lastForward)
    }

    func testWrongHandlerNameIsIgnored() {
        let body: [String: Any] = [
            "id":      "test-wrong-name",
            "request": ["op": "ping"],
        ]
        let message = FakeScriptMessage(name: "wrong_name", body: body)
        handler.userContentController(WKUserContentController(), didReceive: message)

        let exp = expectation(description: "no forward")
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) { exp.fulfill() }
        wait(for: [exp], timeout: 1.0)
        XCTAssertNil(lastForward)
    }

    // MARK: - Concurrent messages

    func testConcurrentBridgeMessages() {
        let group = DispatchGroup()
        var successCount = 0
        let lock = NSLock()

        for i in 0..<20 {
            group.enter()
            DispatchQueue.global().async {
                let body: [String: Any] = [
                    "id":      "concurrent-\(i)",
                    "request": ["op": "ping"],
                ]
                let message = FakeScriptMessage(name: "chimera", body: body)
                self.handler.userContentController(WKUserContentController(), didReceive: message)
                group.leave()
            }
        }

        group.wait()

        // Wait for all bridge dispatches to complete
        let exp = expectation(description: "all responses")
        DispatchQueue.global().asyncAfter(deadline: .now() + 1.0) { exp.fulfill() }
        wait(for: [exp], timeout: 3.0)

        // Verify we got responses (may not get all due to timing)
        XCTAssertNotNil(lastForward)
    }

    // MARK: - ChimeraRequest.fromDictionary

    func testFromDictionaryPing() {
        let dict: [String: Any] = ["op": "ping"]
        let req = ChimeraRequest.fromDictionary(dict)
        XCTAssertNotNil(req)
        if case .ping = req {} else { XCTFail("expected .ping") }
    }

    func testFromDictionaryVersion() {
        let dict: [String: Any] = ["op": "version"]
        let req = ChimeraRequest.fromDictionary(dict)
        XCTAssertNotNil(req)
        if case .version = req {} else { XCTFail("expected .version") }
    }

    func testFromDictionaryListDevices() {
        let dict: [String: Any] = ["op": "list_devices"]
        let req = ChimeraRequest.fromDictionary(dict)
        XCTAssertNotNil(req)
        if case .listDevices = req {} else { XCTFail("expected .listDevices") }
    }

    func testFromDictionaryDrainLogs() {
        let dict: [String: Any] = ["op": "drain_logs"]
        let req = ChimeraRequest.fromDictionary(dict)
        XCTAssertNotNil(req)
        if case .drainLogs = req {} else { XCTFail("expected .drainLogs") }
    }

    func testFromDictionaryValidateImei() {
        let dict: [String: Any] = ["op": "validate_imei", "imei": "123456789012345"]
        let req = ChimeraRequest.fromDictionary(dict)
        XCTAssertNotNil(req)
        if case .validateImei(let imei) = req {
            XCTAssertEqual(imei, "123456789012345")
        } else {
            XCTFail("expected .validateImei")
        }
    }

    func testFromDictionaryValidateMac() {
        let dict: [String: Any] = ["op": "validate_mac", "mac": "AA:BB:CC:DD:EE:FF"]
        let req = ChimeraRequest.fromDictionary(dict)
        XCTAssertNotNil(req)
        if case .validateMac(let mac) = req {
            XCTAssertEqual(mac, "AA:BB:CC:DD:EE:FF")
        } else {
            XCTFail("expected .validateMac")
        }
    }

    func testFromDictionaryUnknownOp() {
        let dict: [String: Any] = ["op": "unknown_operation"]
        let req = ChimeraRequest.fromDictionary(dict)
        XCTAssertNil(req)
    }

    func testFromDictionaryMissingOp() {
        let dict: [String: Any] = ["key": "value"]
        let req = ChimeraRequest.fromDictionary(dict)
        XCTAssertNil(req)
    }
}

// MARK: - Helper

/// XCTest can't directly construct a WKScriptMessage; this stand-in matches
/// the duck-typed properties BridgeMessageHandler reads.
final class FakeScriptMessage: WKScriptMessage {
    private let _name: String
    private let _body: Any

    init(name: String, body: Any) {
        _name = name
        _body = body
        super.init()
    }

    override var name: String { _name }
    override var body: Any { _body }
}
