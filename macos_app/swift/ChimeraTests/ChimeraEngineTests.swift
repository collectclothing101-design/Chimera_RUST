//
//  ChimeraEngineTests.swift
//  XCTest harness for the Swift FFI wrapper.
//
//  Build:
//      swiftc -target arm64-apple-macos10.14 \
//          -import-objc-header ../Chimera/Bridging/Chimera-Bridging-Header.h \
//          -I ../../../crates/chimera-ffi/include \
//          -L ../../../target/aarch64-apple-darwin/debug \
//          -lchimera_ffi -framework XCTest -framework Cocoa \
//          ChimeraEngineTests.swift
//

import XCTest
@testable import Chimera

final class ChimeraEngineTests: XCTestCase {

    override func setUp() {
        super.setUp()
        _ = ChimeraEngine.shared.initialise()
    }

    // MARK: - Sync dispatch

    func testPing() throws {
        let r = try ChimeraEngine.shared.dispatch(.ping)
        if case .err(let m) = r { XCTFail("ping failed: \(m)") }
    }

    func testVersion() {
        let v = ChimeraEngine.shared.version()
        XCTAssertFalse(v.isEmpty, "version string was empty")
    }

    func testVersionViaDispatch() throws {
        let r = try ChimeraEngine.shared.dispatch(.version)
        guard case .ok(let data) = r else { XCTFail("expected ok"); return }
        guard case .object(let obj) = data else { XCTFail("expected object"); return }
        XCTAssertNotNil(obj["name"])
        XCTAssertNotNil(obj["version"])
    }

    // MARK: - Validation

    func testValidImeiPasses() throws {
        let r = try ChimeraEngine.shared.dispatch(
            .validateImei("352099001761481"))
        guard case .ok(let data) = r,
              case .object(let obj) = data,
              case .bool(let valid)? = obj["valid"] else {
            XCTFail("malformed response"); return
        }
        XCTAssertTrue(valid)
    }

    func testInvalidImeiFails() throws {
        let r = try ChimeraEngine.shared.dispatch(.validateImei("not-an-imei"))
        guard case .ok(let data) = r,
              case .object(let obj) = data,
              case .bool(let valid)? = obj["valid"] else {
            XCTFail("malformed response"); return
        }
        XCTAssertFalse(valid)
    }

    func testValidMacPasses() throws {
        let r = try ChimeraEngine.shared.dispatch(.validateMac("aa:bb:cc:dd:ee:ff"))
        guard case .ok(let data) = r,
              case .object(let obj) = data,
              case .bool(let valid)? = obj["valid"] else {
            XCTFail("malformed response"); return
        }
        XCTAssertTrue(valid)
    }

    func testInvalidMacFails() throws {
        let r = try ChimeraEngine.shared.dispatch(.validateMac("invalid"))
        guard case .ok(let data) = r,
              case .object(let obj) = data,
              case .bool(let valid)? = obj["valid"] else {
            XCTFail("malformed response"); return
        }
        XCTAssertFalse(valid)
    }

    // MARK: - Device operations

    func testListDevices() throws {
        let r = try ChimeraEngine.shared.dispatch(.listDevices)
        guard case .ok(let data) = r else { XCTFail("expected ok"); return }
        guard case .array(_) = data else { XCTFail("expected array"); return }
        // May be empty if no devices connected
    }

    func testDrainLogs() throws {
        let r = try ChimeraEngine.shared.dispatch(.drainLogs)
        if case .err(let m) = r { XCTFail("drain_logs failed: \(m)") }
    }

    // MARK: - Async dispatch

    func testAsyncPing() async throws {
        let r = try await ChimeraEngine.shared.dispatchAsync(.ping)
        if case .err(let m) = r { XCTFail("async ping failed: \(m)") }
    }

    func testAsyncVersion() async throws {
        let r = try await ChimeraEngine.shared.dispatchAsync(.version)
        guard case .ok(let data) = r else { XCTFail("expected ok"); return }
        guard case .object(let obj) = data else { XCTFail("expected object"); return }
        XCTAssertNotNil(obj["version"])
    }

    // MARK: - Concurrent stress

    func testConcurrentDispatch() {
        let group = DispatchGroup()
        var failures = 0
        let lock = NSLock()
        for _ in 0..<100 {
            group.enter()
            DispatchQueue.global().async {
                do {
                    _ = try ChimeraEngine.shared.dispatch(.ping)
                } catch {
                    lock.lock(); failures += 1; lock.unlock()
                }
                group.leave()
            }
        }
        group.wait()
        XCTAssertEqual(failures, 0, "concurrent dispatch had \(failures) failures")
    }

    func testConcurrentValidation() {
        let group = DispatchGroup()
        var failures = 0
        let lock = NSLock()
        let imeis = ["352099001761481", "868234020040115", "123456789012345"]
        for _ in 0..<30 {
            group.enter()
            DispatchQueue.global().async {
                for imei in imeis {
                    do {
                        _ = try ChimeraEngine.shared.dispatch(.validateImei(imei))
                    } catch {
                        lock.lock(); failures += 1; lock.unlock()
                    }
                }
                group.leave()
            }
        }
        group.wait()
        XCTAssertEqual(failures, 0, "concurrent validation had \(failures) failures")
    }

    // MARK: - Memory

    func testRepeatedAllocFree() {
        // Hammer the FFI string allocator; should not leak.
        for _ in 0..<10_000 {
            let v = ChimeraEngine.shared.version()
            XCTAssertFalse(v.isEmpty)
        }
    }

    func testRepeatedDispatch() {
        // Repeated dispatch should not leak or crash.
        for _ in 0..<1_000 {
            let r = try? ChimeraEngine.shared.dispatch(.ping)
            XCTAssertNotNil(r)
        }
    }

    // MARK: - Error handling

    func testNotInitialised() {
        let engine = ChimeraEngine()
        // Don't call initialise()
        XCTAssertFalse(engine.isReady)
    }

    func testDoubleInitialise() {
        let r1 = ChimeraEngine.shared.initialise()
        let r2 = ChimeraEngine.shared.initialise()
        XCTAssertTrue(r1)
        XCTAssertTrue(r2)
    }

    func testEngineIsReady() {
        _ = ChimeraEngine.shared.initialise()
        XCTAssertTrue(ChimeraEngine.shared.isReady)
    }
}
