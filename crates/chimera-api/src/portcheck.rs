// chimera-api/src/portcheck.rs
// Replacement for portcheck.chimeratool.com — local TCP port reachability check.
// Original endpoint: GET /check?host={host}&port={port}&proto=tcp
//   → {open: bool, latency_ms: u32, error: string}
// ChimeraRS does this locally without any external server.

use std::net::{TcpStream, SocketAddr};
use std::time::{Duration, Instant};
use log::debug;
use serde::{Deserialize, Serialize};

/// Result of a port check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortCheckResult {
    pub host: String,
    pub port: u16,
    pub open: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

impl PortCheckResult {
    pub fn is_reachable(&self) -> bool { self.open }
}

/// Check if a TCP port is reachable — local equivalent of portcheck.chimeratool.com
pub fn check_tcp_port(host: &str, port: u16, timeout_ms: u64) -> PortCheckResult {
    let addr_str = format!("{}:{}", host, port);
    debug!("portcheck: testing TCP {}:{}", host, port);

    let start = Instant::now();
    let result = addr_str.parse::<SocketAddr>()
        .map_err(|e| format!("Invalid address: {}", e))
        .and_then(|addr| {
            TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms))
                .map_err(|e| e.to_string())
        });

    let latency = start.elapsed().as_millis() as u64;

    match result {
        Ok(_) => PortCheckResult {
            host: host.to_owned(),
            port,
            open: true,
            latency_ms: Some(latency),
            error: None,
        },
        Err(e) => PortCheckResult {
            host: host.to_owned(),
            port,
            open: false,
            latency_ms: Some(latency),
            error: Some(e),
        },
    }
}

/// Check ADB TCP connectivity (default port 5555)
pub fn check_adb_tcp(host: &str) -> PortCheckResult {
    check_tcp_port(host, 5555, 3000)
}

/// Check Fastboot TCP connectivity (default port 5554)
pub fn check_fastboot_tcp(host: &str) -> PortCheckResult {
    check_tcp_port(host, 5554, 3000)
}

/// Check multiple ports in sequence
pub fn check_ports(host: &str, ports: &[u16]) -> Vec<PortCheckResult> {
    ports.iter().map(|&port| check_tcp_port(host, port, 2000)).collect()
}

/// Scan for open ADB devices on a local subnet (e.g. 192.168.1.0/24)
pub fn scan_lan_for_adb(subnet_prefix: &str) -> Vec<PortCheckResult> {
    let mut results = Vec::new();
    for i in 1u8..=254 {
        let host = format!("{}.{}", subnet_prefix, i);
        let r = check_tcp_port(&host, 5555, 500); // Short timeout for scanning
        if r.open {
            results.push(r);
        }
    }
    results
}
