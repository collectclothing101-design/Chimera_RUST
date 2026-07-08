#!/usr/bin/env python3
"""
Mock ADB server for testing ChimeraRS without a real device.
Listens on localhost:5037 and responds to ADB protocol messages.

Usage:
    python3 tools/mock_adb_server.py
    # Then run ChimeraRS — it will connect to this mock automatically.

To simulate different devices, edit the DEVICES dict below.
"""

import socket
import threading
import sys

HOST = "127.0.0.1"
PORT = 5037

# Simulated devices — edit to test different scenarios
DEVICES = {
    "EMULATOR001": {
        "state": "device",
        "model": "Pixel 7",
        "product": "panther",
        "props": {
            "ro.product.model": "Pixel 7",
            "ro.build.PDA": "AP2A.240605.024",
            "ro.build.version.release": "14",
            "ro.build.version.sdk": "34",
            "ro.product.brand": "google",
            "ro.product.manufacturer": "Google",
            "ro.serialno": "EMULATOR001",
            "ro.vendor.warranty.bit": "0",
            "ro.boot.warranty_bit": "0",
            "ro.knox.version": "",
            "ro.csc.sales_code": "GSM",
            "ro.board.platform": "exynos2200",
            "persist.vendor.radio.sim.count": "2",
            "ro.frp.pst": "",
        },
        "imei": "352099001761481",
        "imei2": "352099001761482",
    },
}

# Simulated shell command responses
SHELL_RESPONSES = {
    "getprop ro.product.model": lambda d: d["props"].get("ro.product.model", ""),
    "getprop ro.build.PDA": lambda d: d["props"].get("ro.build.PDA", ""),
    "getprop ro.build.version.release": lambda d: d["props"].get("ro.build.version.release", ""),
    "getprop ro.build.version.sdk": lambda d: d["props"].get("ro.build.version.sdk", ""),
    "getprop ro.product.brand": lambda d: d["props"].get("ro.product.brand", ""),
    "getprop ro.product.manufacturer": lambda d: d["props"].get("ro.product.manufacturer", ""),
    "getprop ro.serialno": lambda d: d["props"].get("ro.serialno", ""),
    "getprop ro.vendor.warranty.bit": lambda d: d["props"].get("ro.vendor.warranty.bit", ""),
    "getprop ro.boot.warranty_bit": lambda d: d["props"].get("ro.boot.warranty_bit", ""),
    "getprop ro.knox.version": lambda d: d["props"].get("ro.knox.version", ""),
    "getprop ro.csc.sales_code": lambda d: d["props"].get("ro.csc.sales_code", ""),
    "getprop ro.board.platform": lambda d: d["props"].get("ro.board.platform", ""),
    "getprop ro.mediatek.platform": lambda d: d["props"].get("ro.mediatek.platform", ""),
    "getprop persist.vendor.radio.sim.count": lambda d: d["props"].get("persist.vendor.radio.sim.count", ""),
    "getprop ro.frp.pst": lambda d: d["props"].get("ro.frp.pst", ""),
    "getprop persist.sys.usb.config": lambda d: "adb",
    "getprop persist.vendor.sys.usb.config": lambda d: "adb",
    "getprop persist.sys.factory_mode": lambda d: "0",
    "getprop persist.ril.id.meid": lambda d: "",
    "getprop persist.sys.battery.serial": lambda d: "",
    "getprop persist.vendor.radio.multisim.config": lambda d: "dsds",
    "getprop persist.sys.usb.vid": lambda d: "18d1",
    "getprop ro.product.cpu.abilist": lambda d: "x86_64,x86",
    "id": lambda d: "uid=0(root) gid=0(root) groups=0(root)",
    "su -c id": lambda d: "uid=0(root) gid=0(root) groups=0(root)",
    "whoami": lambda d: "root",
}

# Track command history for debugging
command_log = []


def handle_getprop(cmd, device):
    """Parse 'getprop KEY' and return the value."""
    parts = cmd.split("getprop ", 1)
    if len(parts) == 2:
        prop = parts[1].strip()
        return device["props"].get(prop, "")
    return ""


def handle_service_call(cmd, device):
    """Simulate 'service call iphonesubinfo N' for IMEI reading."""
    if "iphonesubinfo 1" in cmd:
        # Return IMEI in Android service call format
        imei = device.get("imei", "")
        # Format: Parcel data with hex-encoded characters
        return f"Result: Parcel({imei})"
    elif "iphonesubinfo 3" in cmd:
        imei2 = device.get("imei2", "")
        return f"Result: Parcel({imei2})"
    elif "iphonesubinfo 5" in cmd:
        return "Result: Parcel(00000000)"
    return "Result: Parcel()"


def handle_shell_command(cmd, device):
    """Route a shell command to the appropriate handler."""
    cmd = cmd.strip()

    # getprop
    if cmd.startswith("getprop "):
        return handle_getprop(cmd, device)

    # service call
    if cmd.startswith("service call "):
        return handle_service_call(cmd, device)

    # su -c wrapper
    if cmd.startswith("su -c "):
        inner = cmd[6:].strip().strip("'\"")
        return handle_shell_command(inner, device)

    # id check
    if cmd in ("id", "su -c id", "su -c id 2>&1"):
        return "uid=0(root) gid=0(root) groups=0(root)"

    # wm size
    if cmd == "wm size":
        return "Physical size: 1080x2400"

    # dumpsys battery
    if cmd.startswith("dumpsys battery"):
        return "Current Battery Service state:\n  level: 85\n  temperature: 280\n  status: 2"

    # settings get
    if cmd.startswith("settings get "):
        return ""

    # ls commands
    if cmd.startswith("ls "):
        return "total 0\n"

    # Block destructive commands in mock mode (return success but don't execute)
    destructive = ["rm -rf", "dd if=", "am broadcast", "pm disable", "pm clear",
                   "e2fsck", "wipe", "mount -o"]
    for pattern in destructive:
        if pattern in cmd:
            return ""  # Success, no output

    # Generic success for unknown commands
    return ""


def send_adb_response(stream, data):
    """Send an ADB protocol response: STATUS + optional payload."""
    if isinstance(data, str):
        data = data.encode("utf-8")

    stream.sendall(b"OKAY")
    if data:
        # Send length prefix + data
        length = f"{len(data):04X}".encode()
        stream.sendall(length + data)


def send_adb_fail(stream, message):
    """Send an ADB FAIL response."""
    stream.sendall(b"FAIL")
    msg_bytes = message.encode()
    length = f"{len(msg_bytes):04X}".encode()
    stream.sendall(length + msg_bytes)


def read_adb_message(stream):
    """Read one ADB protocol message: 4-byte hex length + payload."""
    length_hex = stream.recv(4)
    if not length_hex or len(length_hex) < 4:
        return None
    try:
        length = int(length_hex, 16)
    except ValueError:
        return None
    data = b""
    while len(data) < length:
        chunk = stream.recv(length - len(data))
        if not chunk:
            return None
        data += chunk
    return data.decode("utf-8", errors="replace")


def handle_client(conn, addr):
    """Handle one ADB client connection."""
    current_device = None

    try:
        while True:
            # Read command from client
            msg = read_adb_message(conn)
            if msg is None:
                break

            print(f"[{addr}] → {msg[:80]}{'...' if len(msg) > 80 else ''}")
            command_log.append(msg)

            # host:devices-l
            if msg == "host:devices-l":
                device_list = ""
                for serial, info in DEVICES.items():
                    device_list += f"{serial}\t{info['state']}\tmodel:{info['model']},product:{info['product']},transport_id:1\n"
                send_adb_response(conn, device_list)

            # host:transport:SERIAL
            elif msg.startswith("host:transport:"):
                serial = msg.split(":", 2)[2]
                if serial in DEVICES:
                    current_device = serial
                    send_adb_response(conn, b"")
                else:
                    send_adb_fail(conn, f"device '{serial}' not found")

            # shell:COMMAND
            elif msg.startswith("shell:"):
                if current_device and current_device in DEVICES:
                    cmd = msg[6:]
                    device = DEVICES[current_device]
                    output = handle_shell_command(cmd, device)
                    print(f"[{addr}]   ← {output[:80]}{'...' if len(output) > 80 else ''}")
                    send_adb_response(conn, output)
                else:
                    send_adb_fail(conn, "no device selected")

            # host:version
            elif msg == "host:version":
                send_adb_response(conn, b"0029")  # ADB version 39

            # host:features
            elif msg == "host:features":
                send_adb_response(conn, "shell_v2,cmd,stat_v2,ls_v2,fixed_push_mkdir,apex,abb")

            # host:kill
            elif msg == "host:kill":
                send_adb_response(conn, b"")
                print("Server kill requested")
                break

            else:
                # Unknown command — return OKAY with empty response
                send_adb_response(conn, b"")

    except (ConnectionResetError, BrokenPipeError):
        pass
    finally:
        conn.close()
        print(f"[{addr}] disconnected")


def main():
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind((HOST, PORT))
    server.listen(5)

    print(f"╔══════════════════════════════════════════════════════╗")
    print(f"║  Mock ADB Server                                    ║")
    print(f"║  Listening on {HOST}:{PORT}                    ║")
    print(f"║                                                      ║")
    print(f"║  Simulated devices:                                  ║")
    for serial, info in DEVICES.items():
        print(f"║    {serial} ({info['model']})              ║")
    print(f"║                                                      ║")
    print(f"║  Press Ctrl+C to stop                                ║")
    print(f"╚══════════════════════════════════════════════════════╝")
    print()
    print("Run ChimeraRS now — it will connect to this mock.")
    print()

    try:
        while True:
            conn, addr = server.accept()
            thread = threading.Thread(target=handle_client, args=(conn, addr))
            thread.daemon = True
            thread.start()
    except KeyboardInterrupt:
        print("\nShutting down...")
        print(f"\nCommand log ({len(command_log)} commands):")
        for i, cmd in enumerate(command_log[-20:], 1):
            print(f"  {i:3d}. {cmd[:100]}")
        server.close()


if __name__ == "__main__":
    main()
