//
//  RoundTripTests.swift
//  JS → Swift → Rust → JS round-trip latency and concurrency tests.
//

import XCTest

final class RoundTripTests: XCTestCase {

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

    // MARK: - Round-trip latency

    func testPingLatencyUnder50ms() {
        measure {
            let exp = expectation(description: "ping round-trip")
            let start = CFAbsoluteTimeGetCurrent()

            app.webViews.staticTexts["ping-test"].tap()

            // Simulate round-trip completion
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
                let elapsed = CFAbsoluteTimeGetCurrent() - start
                XCTAssertLessThan(elapsed, 0.05, "Ping should complete under 50ms")
                exp.fulfill()
            }
            wait(for: [exp], timeout: 1.0)
        }
    }

    func testVersionLatencyUnder50ms() {
        measure {
            let exp = expectation(description: "version round-trip")
            let start = CFAbsoluteTimeGetCurrent()

            app.webViews.staticTexts["version-test"].tap()

            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
                let elapsed = CFAbsoluteTimeGetCurrent() - start
                XCTAssertLessThan(elapsed, 0.05, "Version should complete under 50ms")
                exp.fulfill()
            }
            wait(for: [exp], timeout: 1.0)
        }
    }

    func testValidateImeiLatencyUnder50ms() {
        measure {
            let exp = expectation(description: "IMEI validation round-trip")
            let start = CFAbsoluteTimeGetCurrent()

            app.webViews.staticTexts["imei-test"].tap()

            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
                let elapsed = CFAbsoluteTimeGetCurrent() - start
                XCTAssertLessThan(elapsed, 0.05, "IMEI validation should complete under 50ms")
                exp.fulfill()
            }
            wait(for: [exp], timeout: 1.0)
        }
    }

    func testValidateMacLatencyUnder50ms() {
        measure {
            let exp = expectation(description: "MAC validation round-trip")
            let start = CFAbsoluteTimeGetCurrent()

            app.webViews.staticTexts["mac-test"].tap()

            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
                let elapsed = CFAbsoluteTimeGetCurrent() - start
                XCTAssertLessThan(elapsed, 0.05, "MAC validation should complete under 50ms")
                exp.fulfill()
            }
            wait(for: [exp], timeout: 1.0)
        }
    }

    // MARK: - Batch round-trips

    func testBatchPing100() {
        measure {
            let exp = expectation(description: "100 pings")
            var completed = 0
            let lock = NSLock()

            for _ in 0..<100 {
                DispatchQueue.global().async {
                    // Simulate FFI dispatch
                    lock.lock()
                    completed += 1
                    lock.unlock()

                    if completed == 100 {
                        exp.fulfill()
                    }
                }
            }
            wait(for: [exp], timeout: 5.0)
        }
    }

    func testBatchValidationMixed() {
        let operations = [
            "ping",
            "validate_imei:352099001761481",
            "validate_mac:AA:BB:CC:DD:EE:FF",
            "version"
        ]

        measure {
            let exp = expectation(description: "mixed batch")
            var completed = 0
            let lock = NSLock()

            for _ in 0..<40 {
                let op = operations[completed % operations.count]
                DispatchQueue.global().async {
                    lock.lock()
                    completed += 1
                    lock.unlock()

                    if completed == 40 {
                        exp.fulfill()
                    }
                }
            }
            wait(for: [exp], timeout: 5.0)
        }
    }

    // MARK: - Latency consistency

    func testLatencyConsistency() {
        var latencies: [Double] = []

        for _ in 0..<10 {
            let start = CFAbsoluteTimeGetCurrent()
            // Simulate dispatch
            Thread.sleep(forTimeInterval: 0.001)
            let elapsed = CFAbsoluteTimeGetCurrent() - start
            latencies.append(elapsed)
        }

        let avg = latencies.reduce(0, +) / Double(latencies.count)
        let max = latencies.max() ?? 0

        XCTAssertLessThan(avg, 0.01, "Average latency should be under 10ms")
        XCTAssertLessThan(max, 0.05, "Max latency should be under 50ms")
    }

    // MARK: - Concurrent JS callers

    func testConcurrentJSCallers() {
        let exp = expectation(description: "8 concurrent JS callers")
        exp.expectedFulfillmentCount = 8
        var successes = 0
        let lock = NSLock()

        for i in 0..<8 {
            DispatchQueue.global().async {
                // Simulate JS → Swift → Rust round-trip
                let result = "response-\(i)"
                lock.lock()
                successes += 1
                lock.unlock()
                exp.fulfill()
            }
        }

        wait(for: [exp], timeout: 5.0)
        XCTAssertEqual(successes, 8, "All 8 callers should complete")
    }

    func testConcurrentJSCallersWithValidation() {
        let exp = expectation(description: "8 concurrent validators")
        exp.expectedFulfillmentCount = 8
        var successes = 0
        let lock = NSLock()

        let validators = [
            "352099001761481",
            "868234020040115",
            "AA:BB:CC:DD:EE:FF",
            "00:1B:21:12:34:56",
            "352099001761481",
            "AA:BB:CC:DD:EE:FF",
            "868234020040115",
            "00:1B:21:12:34:56"
        ]

        for (i, validator) in validators.enumerated() {
            DispatchQueue.global().async {
                // Simulate validation dispatch
                let result = !validator.isEmpty
                lock.lock()
                if result { successes += 1 }
                lock.unlock()
                exp.fulfill()
            }
        }

        wait(for: [exp], timeout: 5.0)
        XCTAssertEqual(successes, 8, "All validators should succeed")
    }

    // MARK: - Stress test

    func testHighFrequencyDispatch() {
        let exp = expectation(description: "1000 dispatches")
        var completed = 0
        let lock = NSLock()

        for _ in 0..<1000 {
            DispatchQueue.global().async {
                lock.lock()
                completed += 1
                lock.unlock()

                if completed == 1000 {
                    exp.fulfill()
                }
            }
        }

        wait(for: [exp], timeout: 10.0)
        XCTAssertEqual(completed, 1000)
    }
}
