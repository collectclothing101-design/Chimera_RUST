// chimera-api/src/pics_api.rs
// Replacement for pics.chimeratool.com device image CDN.
// Fetches device photos from open sources and caches locally.

use std::path::PathBuf;
use log::{info, debug};

/// Local cache directory for device images
/// macOS: ~/Library/Caches/chimera-rs/pics
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()  // macOS: ~/Library/Caches
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Library")
                .join("Caches")
        })
        .join("chimera-rs")
        .join("pics")
}

/// Open image sources (in priority order)
pub const IMAGE_SOURCES: &[(&str, &str)] = &[
    ("gsmarena",  "https://cdn2.gsmarena.com/vv/bigpic/{slug}.jpg"),
    ("devicedb",  "https://storage.devicedb.io/images/{model}.jpg"),
    ("techspecs", "https://techspecs.io/api/device/image?model={model}"),
];

/// Get a device image path (from cache or download)
pub async fn get_device_image(model_identifier: &str, model_name: &str) -> Option<PathBuf> {
    let cache = cache_dir();
    std::fs::create_dir_all(&cache).ok()?;

    // Check cache first
    let cached = cache.join(format!("{}.jpg", model_identifier));
    if cached.exists() {
        debug!("Device image cache hit: {}", model_identifier);
        return Some(cached);
    }

    // Attempt to fetch from open sources
    let slug = model_name
        .to_lowercase()
        .replace(' ', "-")
        .replace('/', "-");

    // Try each source in priority order
    for (source_name, url_template) in IMAGE_SOURCES {
        let url = url_template
            .replace("{model}", model_identifier)
            .replace("{slug}", &slug);
        info!("Fetching device image from {}: {}", source_name, url);

        // Use a blocking reqwest client to download the image
        match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("ChimeraRS/1.0")
            .build()
        {
            Ok(client) => {
                match client.get(&url).send() {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.bytes() {
                            Ok(bytes) if !bytes.is_empty() => {
                                if let Ok(()) = std::fs::write(&cached, &bytes) {
                                    info!("Device image cached: {} from {}", model_identifier, source_name);
                                    return Some(cached);
                                }
                            }
                            _ => debug!("Empty response from {}", source_name),
                        }
                    }
                    Ok(resp) => debug!("HTTP {} from {}: {}", resp.status(), source_name, url),
                    Err(e) => debug!("Request to {} failed: {}", source_name, e),
                }
            }
            Err(e) => debug!("HTTP client build error: {}", e),
        }
    }

    debug!("No device image found for {} from any source", model_identifier);
    None
}

/// Brand logo image (fallback to embedded egui icons)
pub fn brand_logo_key(brand: &str) -> &'static str {
    match brand.to_lowercase().as_str() {
        "samsung"               => "samsung_logo",
        "apple" | "iphone"      => "apple_logo",
        "xiaomi" | "poco"       => "xiaomi_logo",
        "huawei"                => "huawei_logo",
        "motorola"              => "motorola_logo",
        "lg"                    => "lg_logo",
        "sony"                  => "sony_logo",
        "nokia"                 => "nokia_logo",
        "oppo" | "realme"       => "oppo_logo",
        "oneplus"               => "oneplus_logo",
        "nothing"               => "nothing_logo",
        _                       => "generic_phone",
    }
}
