// chimera-utils/src/qr_code.rs
// QR code generation for ADB wireless pairing
use chimera_core::error::{ChimeraError, Result};

pub struct QrCodeGenerator;

impl QrCodeGenerator {
    /// Generate QR code for ADB wireless pairing (Android 11+)
    pub fn generate_adb_pair_qr(service_name: &str, password: &str) -> Result<Vec<u8>> {
        // Wi-Fi Aware pairing format used by Android 11+ wireless ADB
        let qr_data = format!("WIFI:T:ADB;S:{};P:{};;", service_name, password);
        
        let code = qrcode::QrCode::new(qr_data.as_bytes())
            .map_err(|e| ChimeraError::Unknown(format!("QR generation error: {}", e)))?;
        
        // Render as PNG
        let image = code.render::<image::Luma<u8>>()
            .min_dimensions(200, 200)
            .build();
        
        let mut png_bytes = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .map_err(|e| ChimeraError::Unknown(format!("PNG encode error: {}", e)))?;
        
        Ok(png_bytes)
    }

    /// Generate QR code data as ASCII art string
    pub fn generate_ascii(data: &str) -> Result<String> {
        let code = qrcode::QrCode::new(data.as_bytes())
            .map_err(|e| ChimeraError::Unknown(format!("QR error: {}", e)))?;
        Ok(code.render::<char>()
            .quiet_zone(false)
            .module_dimensions(1, 1)
            .build())
    }

    /// Generate QR code as a PNG byte vector for arbitrary text. Used by
    /// the FFI's `generate_qr` op so the WKWebView UI can render a real,
    /// spec-compliant QR code (the inline JS fallback is decorative).
    pub fn generate_png(data: &str, min_dimension: u32) -> Result<Vec<u8>> {
        let code = qrcode::QrCode::new(data.as_bytes())
            .map_err(|e| ChimeraError::Unknown(format!("QR generation error: {}", e)))?;
        let image = code.render::<image::Luma<u8>>()
            .min_dimensions(min_dimension, min_dimension)
            .build();
        let mut png_bytes = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .map_err(|e| ChimeraError::Unknown(format!("PNG encode error: {}", e)))?;
        Ok(png_bytes)
    }
}
