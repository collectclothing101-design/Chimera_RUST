//
//  JSONValueTests.swift
//  XCTest harness for JSONValue Codable round-trips.
//

import XCTest
@testable import Chimera

final class JSONValueTests: XCTestCase {

    // MARK: - Null

    func testNullEncodeDecode() throws {
        let original: JSONValue = .null
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .null = decoded {} else { XCTFail("expected .null") }
    }

    func testNullFoundationObject() {
        let value: JSONValue = .null
        let obj = value.foundationObject
        XCTAssertTrue(obj is NSNull)
    }

    // MARK: - Bool

    func testBoolTrueEncodeDecode() throws {
        let original: JSONValue = .bool(true)
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .bool(let b) = decoded {
            XCTAssertTrue(b)
        } else {
            XCTFail("expected .bool")
        }
    }

    func testBoolFalseEncodeDecode() throws {
        let original: JSONValue = .bool(false)
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .bool(let b) = decoded {
            XCTAssertFalse(b)
        } else {
            XCTFail("expected .bool")
        }
    }

    func testBoolFoundationObject() {
        XCTAssertTrue(JSONValue.bool(true).foundationObject as? Bool == true)
        XCTAssertTrue(JSONValue.bool(false).foundationObject as? Bool == false)
    }

    // MARK: - Number

    func testNumberEncodeDecode() throws {
        let original: JSONValue = .number(42.5)
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .number(let n) = decoded {
            XCTAssertEqual(n, 42.5, accuracy: 0.001)
        } else {
            XCTFail("expected .number")
        }
    }

    func testNumberZero() throws {
        let original: JSONValue = .number(0)
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .number(let n) = decoded {
            XCTAssertEqual(n, 0)
        } else {
            XCTFail("expected .number")
        }
    }

    func testNumberNegative() throws {
        let original: JSONValue = .number(-123.456)
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .number(let n) = decoded {
            XCTAssertEqual(n, -123.456, accuracy: 0.001)
        } else {
            XCTFail("expected .number")
        }
    }

    func testNumberFoundationObject() {
        let obj = JSONValue.number(42.5).foundationObject
        XCTAssertEqual(obj as? Double, 42.5)
    }

    // MARK: - String

    func testStringEncodeDecode() throws {
        let original: JSONValue = .string("hello world")
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .string(let s) = decoded {
            XCTAssertEqual(s, "hello world")
        } else {
            XCTFail("expected .string")
        }
    }

    func testStringEmpty() throws {
        let original: JSONValue = .string("")
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .string(let s) = decoded {
            XCTAssertEqual(s, "")
        } else {
            XCTFail("expected .string")
        }
    }

    func testStringSpecialChars() throws {
        let original: JSONValue = .string("hello\n\t\"world\\")
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .string(let s) = decoded {
            XCTAssertEqual(s, "hello\n\t\"world\\")
        } else {
            XCTFail("expected .string")
        }
    }

    func testStringUnicode() throws {
        let original: JSONValue = .string("🎉🔥💀")
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .string(let s) = decoded {
            XCTAssertEqual(s, "🎉🔥💀")
        } else {
            XCTFail("expected .string")
        }
    }

    func testStringFoundationObject() {
        let obj = JSONValue.string("test").foundationObject
        XCTAssertEqual(obj as? String, "test")
    }

    // MARK: - Array

    func testArrayEncodeDecode() throws {
        let original: JSONValue = .array([.number(1), .number(2), .number(3)])
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .array(let arr) = decoded {
            XCTAssertEqual(arr.count, 3)
        } else {
            XCTFail("expected .array")
        }
    }

    func testArrayEmpty() throws {
        let original: JSONValue = .array([])
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .array(let arr) = decoded {
            XCTAssertEqual(arr.count, 0)
        } else {
            XCTFail("expected .array")
        }
    }

    func testArrayMixedTypes() throws {
        let original: JSONValue = .array([
            .null,
            .bool(true),
            .number(42),
            .string("hello"),
            .array([.number(1)]),
            .object(["key": .string("value")])
        ])
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .array(let arr) = decoded {
            XCTAssertEqual(arr.count, 6)
        } else {
            XCTFail("expected .array")
        }
    }

    func testArrayFoundationObject() {
        let arr: JSONValue = .array([.number(1), .string("two")])
        let obj = arr.foundationObject as? [Any]
        XCTAssertNotNil(obj)
        XCTAssertEqual(obj?.count, 2)
    }

    // MARK: - Object

    func testObjectEncodeDecode() throws {
        let original: JSONValue = .object([
            "name": .string("Chimera"),
            "version": .string("1.0.0")
        ])
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .object(let obj) = decoded {
            XCTAssertNotNil(obj["name"])
            XCTAssertNotNil(obj["version"])
        } else {
            XCTFail("expected .object")
        }
    }

    func testObjectEmpty() throws {
        let original: JSONValue = .object([:])
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .object(let obj) = decoded {
            XCTAssertEqual(obj.count, 0)
        } else {
            XCTFail("expected .object")
        }
    }

    func testObjectNested() throws {
        let original: JSONValue = .object([
            "outer": .object([
                "inner": .string("value")
            ])
        ])
        let data = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(JSONValue.self, from: data)
        if case .object(let obj) = decoded,
           case .object(let inner)? = obj["outer"] {
            XCTAssertEqual(inner["inner"], .string("value"))
        } else {
            XCTFail("expected nested object")
        }
    }

    func testObjectFoundationObject() {
        let obj: JSONValue = .object([
            "key": .string("value"),
            "num": .number(42)
        ])
        let foundation = obj.foundationObject as? [String: Any]
        XCTAssertNotNil(foundation)
        XCTAssertEqual(foundation?["key"] as? String, "value")
        XCTAssertEqual(foundation?["num"] as? Double, 42)
    }

    // MARK: - ChimeraResponse

    func testChimeraResponseOkDecode() throws {
        let json = """
        {"status":"ok","data":{"name":"ChimeraRS","version":"1.0.0"}}
        """
        let data = json.data(using: .utf8)!
        let response = try JSONDecoder().decode(ChimeraResponse.self, from: data)
        if case .ok(let data) = response,
           case .object(let obj) = data {
            XCTAssertEqual(obj["name"], .string("ChimeraRS"))
            XCTAssertEqual(obj["version"], .string("1.0.0"))
        } else {
            XCTFail("expected .ok with object")
        }
    }

    func testChimeraResponseErrDecode() throws {
        let json = """
        {"status":"err","message":"device not found"}
        """
        let data = json.data(using: .utf8)!
        let response = try JSONDecoder().decode(ChimeraResponse.self, from: data)
        if case .err(let msg) = response {
            XCTAssertEqual(msg, "device not found")
        } else {
            XCTFail("expected .err")
        }
    }

    func testChimeraResponseUnknownStatus() {
        let json = """
        {"status":"unknown"}
        """
        let data = json.data(using: .utf8)!
        XCTAssertThrowsError(try JSONDecoder().decode(ChimeraResponse.self, from: data))
    }

    // MARK: - ChimeraRequest encoding

    func testRequestPingEncoding() throws {
        let request = ChimeraRequest.ping
        let data = try JSONEncoder().encode(request)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(json?["op"] as? String, "ping")
    }

    func testRequestVersionEncoding() throws {
        let request = ChimeraRequest.version
        let data = try JSONEncoder().encode(request)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(json?["op"] as? String, "version")
    }

    func testRequestListDevicesEncoding() throws {
        let request = ChimeraRequest.listDevices
        let data = try JSONEncoder().encode(request)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(json?["op"] as? String, "list_devices")
    }

    func testRequestDrainLogsEncoding() throws {
        let request = ChimeraRequest.drainLogs
        let data = try JSONEncoder().encode(request)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(json?["op"] as? String, "drain_logs")
    }

    func testRequestValidateImeiEncoding() throws {
        let request = ChimeraRequest.validateImei("352099001761481")
        let data = try JSONEncoder().encode(request)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(json?["op"] as? String, "validate_imei")
        XCTAssertEqual(json?["imei"] as? String, "352099001761481")
    }

    func testRequestValidateMacEncoding() throws {
        let request = ChimeraRequest.validateMac("AA:BB:CC:DD:EE:FF")
        let data = try JSONEncoder().encode(request)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(json?["op"] as? String, "validate_mac")
        XCTAssertEqual(json?["mac"] as? String, "AA:BB:CC:DD:EE:FF")
    }
}
