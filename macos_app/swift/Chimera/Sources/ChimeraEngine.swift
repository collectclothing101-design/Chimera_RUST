//
//  ChimeraEngine.swift
//  Type-safe Swift wrapper around the `chimera_ffi` C ABI.
//
//  Pattern:
//      let engine = ChimeraEngine.shared
//      engine.initialise()
//      let response: ChimeraResponse = try engine.dispatch(.ping)
//
//  Threading:
//      The Rust engine is internally thread-safe (Mutex around state).
//      Swift callers may invoke from any queue; ChimeraEngine serialises
//      access via its own internal lock to avoid concurrent FFI calls
//      that would interleave string-marshalling.
//

import Foundation

// MARK: - Request / Response types

/// A request the Swift host sends to the Rust engine.
public enum ChimeraRequest: Encodable {
    case ping
    case version
    case listDevices
    case validateImei(String)
    case validateMac(String)
    case validateIpsw(URL)
    case drainLogs

    private enum CodingKeys: String, CodingKey {
        case op
        case imei
        case mac
        case path
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .ping:           try c.encode("ping", forKey: .op)
        case .version:        try c.encode("version", forKey: .op)
        case .listDevices:    try c.encode("list_devices", forKey: .op)
        case .drainLogs:      try c.encode("drain_logs", forKey: .op)
        case .validateImei(let imei):
            try c.encode("validate_imei", forKey: .op)
            try c.encode(imei,            forKey: .imei)
        case .validateMac(let mac):
            try c.encode("validate_mac", forKey: .op)
            try c.encode(mac,            forKey: .mac)
        case .validateIpsw(let url):
            try c.encode("validate_ipsw", forKey: .op)
            try c.encode(url.path,        forKey: .path)
        }
    }
}

/// A response from the Rust engine.
public enum ChimeraResponse: Decodable {
    case ok(data: JSONValue)
    case err(message: String)

    private enum CodingKeys: String, CodingKey {
        case status, data, message
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let status = try c.decode(String.self, forKey: .status)
        switch status {
        case "ok":
            let data = try c.decode(JSONValue.self, forKey: .data)
            self = .ok(data: data)
        case "err":
            let msg = try c.decode(String.self, forKey: .message)
            self = .err(message: msg)
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .status, in: c,
                debugDescription: "unknown status: \(status)")
        }
    }
}

/// A generic JSON value Swift can pass to / from JavaScript / SwiftUI.
public indirect enum JSONValue: Codable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    public init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() { self = .null; return }
        if let b = try? c.decode(Bool.self)   { self = .bool(b); return }
        if let n = try? c.decode(Double.self) { self = .number(n); return }
        if let s = try? c.decode(String.self) { self = .string(s); return }
        if let a = try? c.decode([JSONValue].self) { self = .array(a); return }
        if let o = try? c.decode([String: JSONValue].self) { self = .object(o); return }
        throw DecodingError.dataCorruptedError(
            in: c, debugDescription: "JSONValue: unsupported type")
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        switch self {
        case .null:           try c.encodeNil()
        case .bool(let b):    try c.encode(b)
        case .number(let n):  try c.encode(n)
        case .string(let s):  try c.encode(s)
        case .array(let a):   try c.encode(a)
        case .object(let o):  try c.encode(o)
        }
    }

    /// Re-serialise into a JSON-compatible Foundation object suitable for
    /// passing to WKWebView via evaluateJavaScript.
    public var foundationObject: Any {
        switch self {
        case .null:           return NSNull()
        case .bool(let b):    return b
        case .number(let n):  return n
        case .string(let s):  return s
        case .array(let a):   return a.map { $0.foundationObject }
        case .object(let o):
            var dict = [String: Any]()
            for (k, v) in o { dict[k] = v.foundationObject }
            return dict
        }
    }
}

// MARK: - Errors

public enum ChimeraEngineError: Error, CustomStringConvertible {
    case notInitialised
    case ffiReturnedNull
    case encodingFailed(Error)
    case decodingFailed(Error)
    case engine(String)

    public var description: String {
        switch self {
        case .notInitialised:        return "ChimeraEngine: chimera_init failed"
        case .ffiReturnedNull:       return "ChimeraEngine: FFI returned NULL"
        case .encodingFailed(let e): return "ChimeraEngine: encode request: \(e)"
        case .decodingFailed(let e): return "ChimeraEngine: decode response: \(e)"
        case .engine(let m):         return "ChimeraEngine: \(m)"
        }
    }
}

// MARK: - Engine wrapper

public final class ChimeraEngine {

    public static let shared = ChimeraEngine()

    private let lock      = NSLock()
    private let encoder   = JSONEncoder()
    private let decoder   = JSONDecoder()
    private(set) var isReady = false

    private init() {}

    /// Initialise the Rust engine. Safe to call multiple times.
    @discardableResult
    public func initialise() -> Bool {
        lock.lock(); defer { lock.unlock() }
        if isReady { return true }
        let rc = chimera_init()
        isReady = (rc == 0)
        return isReady
    }

    /// Synchronous engine version lookup.
    public func version() -> String {
        guard let ptr = chimera_version() else { return "unknown" }
        defer { chimera_string_free(ptr) }
        return String(cString: ptr)
    }

    /// Send a typed request, receive a typed response.
    public func dispatch(_ request: ChimeraRequest) throws -> ChimeraResponse {
        if !isReady, !initialise() { throw ChimeraEngineError.notInitialised }
        lock.lock(); defer { lock.unlock() }

        let payload: Data
        do {
            payload = try encoder.encode(request)
        } catch {
            throw ChimeraEngineError.encodingFailed(error)
        }
        guard let requestString = String(data: payload, encoding: .utf8) else {
            throw ChimeraEngineError.encodingFailed(
                NSError(domain: "ChimeraEngine", code: -1))
        }

        let responsePtr: UnsafeMutablePointer<CChar>? = requestString
            .withCString { chimera_dispatch($0) }
        guard let ptr = responsePtr else { throw ChimeraEngineError.ffiReturnedNull }
        defer { chimera_string_free(ptr) }

        let responseString = String(cString: ptr)
        guard let data = responseString.data(using: .utf8) else {
            throw ChimeraEngineError.engine("non-UTF8 response")
        }
        do {
            return try decoder.decode(ChimeraResponse.self, from: data)
        } catch {
            throw ChimeraEngineError.decodingFailed(error)
        }
    }

    /// Async wrapper for use from MainActor / Task contexts.
    /// Requires macOS 10.15+ for Swift concurrency.
    @available(macOS 10.15, *)
    public func dispatchAsync(_ request: ChimeraRequest) async throws -> ChimeraResponse {
        try await withCheckedThrowingContinuation { cont in
            DispatchQueue.global(qos: .userInitiated).async {
                do {
                    let r = try self.dispatch(request)
                    cont.resume(returning: r)
                } catch {
                    cont.resume(throwing: error)
                }
            }
        }
    }
}
