// crates/chimera-gui/src/main.rs
#![allow(clippy::collapsible_if)]

mod app;
mod history;
mod icons;
mod local_event;
mod persistence;
mod state;
mod theme;
mod ui;
mod worker;


use chimera_core::{APP_NAME, VERSION};
use crossbeam_channel::unbounded;
use eframe::egui;
use log::{error, info};

use crate::local_event::LocalEvent;
use crate::worker::WorkerPool;

fn main() -> eframe::Result<()> {
    init_logging();

    info!(
        "Starting {} v{} on macOS x86_64",
        APP_NAME,
        VERSION
    );

    // ---------------------------------------------------------------------
    // Shared channels
    // ---------------------------------------------------------------------

    let (event_tx, _event_rx) =
        unbounded::<chimera_core::event::ChimeraEvent>();

    let (local_tx, _local_rx) =
        unbounded::<LocalEvent>();

    // ---------------------------------------------------------------------
    // Worker pool
    // ---------------------------------------------------------------------

    let pool = WorkerPool::new(
        event_tx.clone(),
        local_tx.clone(),
    );

    let op_tx = pool.sender();

    // Start background workers
    let _worker_thread = pool.start();

    // ---------------------------------------------------------------------
    // Native window options
    // ---------------------------------------------------------------------

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!(
                "{} v{} — Mobile Repair Tool",
                APP_NAME,
                VERSION
            ))
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([960.0, 640.0])
            .with_icon(load_icon()),

        centered: true,

        ..Default::default()
    };

    // ---------------------------------------------------------------------
    // Launch GUI
    // ---------------------------------------------------------------------

    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(
                app::ChimeraApp::new(cc, op_tx.clone())
            ))
        }),
    )
}

// ───────────────────────────────────────────────────────────────────────────
// Logging
// ───────────────────────────────────────────────────────────────────────────

fn init_logging() {
    let filter = std::env::var("CHIMERA_LOG")
        .unwrap_or_else(|_| "info".to_string());

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::new(filter)
        )
        .try_init();
}

// ───────────────────────────────────────────────────────────────────────────
// Icon loader
// ───────────────────────────────────────────────────────────────────────────

fn load_icon() -> egui::IconData {
    match try_load_icon() {
        Ok(bytes) => {
            // Expecting RGBA8888 32x32
            const SIZE: u32 = 32;

            let expected = (SIZE * SIZE * 4) as usize;

            if bytes.len() != expected {
                error!(
                    "Invalid icon size: expected {} bytes, got {}",
                    expected,
                    bytes.len()
                );

                fallback_icon()
            } else {
                egui::IconData {
                    rgba: bytes,
                    width: SIZE,
                    height: SIZE,
                }
            }
        }

        Err(err) => {
            error!("Failed to load icon: {}", err);
            fallback_icon()
        }
    }
}

fn fallback_icon() -> egui::IconData {
    egui::IconData {
        rgba: vec![0, 0, 0, 0],
        width: 1,
        height: 1,
    }
}

fn try_load_icon() -> std::io::Result<Vec<u8>> {
    let path = std::path::Path::new(
        env!("CARGO_MANIFEST_DIR")
    )
    .join("assets")
    .join("icon.rgba");

    std::fs::read(path)
}