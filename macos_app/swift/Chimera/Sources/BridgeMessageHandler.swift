//
//  BridgeMessageHandler.swift
//
//  Receives WKScriptMessage events from the bundled HTML's JavaScript,
//  validates + sanitises them, dispatches into the Rust engine on a
//  background queue, and forwards the JSON response back to the page.
//
//  Protocol:
//      JS  → Swift:  window.webkit.messageHandlers.chimera.postMessage({
//          id: "<uuid>",
//          request: { op: "<operation>", ... }
//      });
//      Rust → JS:    ChimeraBridge._receive('{"id":"<uuid>","response":{...}}')
//

import Foundation
import WebKit

final class BridgeMessageHandler: NSObject, WKScriptMessageHandler {

    /// Callback the controller uses to forward a JSON string back to JS via
    /// `evaluateJavaScript`. The string is already a complete JS-literal-safe
    /// JSON envelope; the caller wraps it as `ChimeraBridge._receive(<...>)`.
    private let send: (String) -> Void

    /// Dedicated background queue so concurrent JS calls don't block the main
    /// thread. The Rust engine internally serialises FFI access so this only
    /// limits Swift-side dispatch overhead.
    private let queue = DispatchQueue(
        label: "io.chimerars.bridge",
        qos:   .userInitiated,
        attributes: .concurrent)

    init(send: @escaping (String) -> Void) {
        self.send = send
        super.init()
    }

    // MARK: - WKScriptMessageHandler

    func userContentController(_ userContentController: WKUserContentController,
                               didReceive message: WKScriptMessage) {
        guard message.name == "chimera" else { return }

        // Validate envelope ─────────────────────────────────────────
        guard let body = message.body as? [String: Any],
              let id   = body["id"] as? String,
              let req  = body["request"] as? [String: Any] else {
            NSLog("[Chimera] bridge: malformed envelope from JS")
            return
        }

        // Re-serialise the request → JSON the FFI accepts.
        guard let requestData = try? JSONSerialization.data(withJSONObject: req,
                                                            options: []),
              let requestJSON = String(data: requestData, encoding: .utf8) else {
            replyError(id: id, message: "bridge: serialise request")
            return
        }

        queue.async { [weak self] in
            guard let self = self else { return }
            self.dispatchAndReply(id: id, requestJSON: requestJSON, raw: req)
        }
    }

    // MARK: - Dispatch

    private func dispatchAndReply(id: String,
                                  requestJSON: String,
                                  raw: [String: Any]) {
        // Path 1: structured ChimeraRequest the Swift wrapper knows about.
        // Path 2: raw passthrough — useful for forward-compat ops the Swift
        //         layer hasn't typed yet (the FFI still understands them).
        let typed = ChimeraRequest.fromDictionary(raw)

        let response: ChimeraResponse
        do {
            if let typed = typed {
                response = try ChimeraEngine.shared.dispatch(typed)
            } else {
                // Raw passthrough — bypass the typed encoder.
                response = try dispatchRaw(requestJSON: requestJSON)
            }
        } catch {
            replyError(id: id, message: "\(error)")
            return
        }

        // Forward the response back to JS.
        let payload: [String: Any]
        switch response {
        case .ok(let data):
            payload = ["id": id, "response": [
                "status": "ok",
                "data":   data.foundationObject,
            ]]
        case .err(let msg):
            payload = ["id": id, "response": [
                "status":  "err",
                "message": msg,
            ]]
        }
        send(payload: payload)
    }

    /// Bypass the typed Codable layer and call the FFI with the JSON exactly
    /// as the JS produced it. Used for forward-compatible operations.
    private func dispatchRaw(requestJSON: String) throws -> ChimeraResponse {
        let ptr = requestJSON.withCString { chimera_dispatch($0) }
        guard let ptr = ptr else { throw ChimeraEngineError.ffiReturnedNull }
        defer { chimera_string_free(ptr) }
        let responseString = String(cString: ptr)
        guard let data = responseString.data(using: .utf8) else {
            throw ChimeraEngineError.engine("non-UTF8 raw response")
        }
        do {
            return try JSONDecoder().decode(ChimeraResponse.self, from: data)
        } catch {
            throw ChimeraEngineError.decodingFailed(error)
        }
    }

    // MARK: - Helpers

    private func replyError(id: String, message: String) {
        send(payload: ["id": id, "response": [
            "status":  "err",
            "message": message,
        ]])
    }

    private func send(payload: [String: Any]) {
        guard let data = try? JSONSerialization.data(withJSONObject: payload,
                                                     options: []),
              let json = String(data: data, encoding: .utf8) else {
            NSLog("[Chimera] bridge: failed to serialise response")
            return
        }
        send(json)
    }
}

// MARK: - ChimeraRequest dict bridge

extension ChimeraRequest {

    /// Map a JS-side dictionary to a typed Swift request. Returns nil if the
    /// op isn't one the typed layer recognises; callers fall back to raw
    /// FFI passthrough.
    static func fromDictionary(_ dict: [String: Any]) -> ChimeraRequest? {
        guard let op = dict["op"] as? String else { return nil }
        switch op {
        case "ping":         return .ping
        case "version":      return .version
        case "list_devices": return .listDevices
        case "drain_logs":   return .drainLogs
        case "validate_imei":
            guard let imei = dict["imei"] as? String else { return nil }
            return .validateImei(imei)
        case "validate_mac":
            guard let mac = dict["mac"] as? String else { return nil }
            return .validateMac(mac)
        case "validate_ipsw":
            guard let path = dict["path"] as? String else { return nil }
            return .validateIpsw(URL(fileURLWithPath: path))
        default:
            return nil
        }
    }
}
