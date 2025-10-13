// utils/image_utils.rs
use std::env;
use uuid::Uuid;

pub async fn upload_image(base64_data: &str, folder: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Remove data URL prefix if present
    let clean_data = if base64_data.starts_with("data:image") {
        base64_data.split(',').nth(1).unwrap_or(base64_data)
    } else {
        base64_data
    };

    // Decode base64
    let image_data = base64::decode(clean_data)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // Generate unique filename
    let filename = format!("{}/{}.jpg", folder, Uuid::new_v4());
    
    // In production, you would upload to cloud storage (AWS S3, Google Cloud Storage, etc.)
    // For now, we'll return a placeholder URL
    let storage_url = env::var("STORAGE_BASE_URL").unwrap_or_else(|_| "https://storage.verinest.com".to_string());
    
    Ok(format!("{}/{}", storage_url, filename))
}

pub fn validate_image_size(base64_data: &str, max_size_mb: usize) -> Result<bool, String> {
    let clean_data = if base64_data.starts_with("data:image") {
        base64_data.split(',').nth(1).unwrap_or(base64_data)
    } else {
        base64_data
    };

    let size_in_bytes = (clean_data.len() * 3) / 4; // Approximate base64 size
    let max_size_bytes = max_size_mb * 1024 * 1024;
    
    Ok(size_in_bytes <= max_size_bytes)
}