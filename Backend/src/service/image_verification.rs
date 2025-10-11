// src/service/image_verification.rs
use std::error::Error;
use std::fmt;
use std::collections::HashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use image::{ImageFormat, DynamicImage, GenericImageView};
use std::io::Cursor;
use base64::{Engine as _, engine::general_purpose};

// Custom error types for image verification
#[derive(Debug)]
pub enum ImageVerificationError {
    InvalidFormat(String),
    FileSizeExceeded(String),
    ProcessingError(String),
    ApiError(String),
    NetworkError(String),
    DuplicateImage(String),
    QualityInsufficient(String),
}

impl fmt::Display for ImageVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ImageVerificationError::InvalidFormat(msg) => write!(f, "Invalid image format: {}", msg),
            ImageVerificationError::FileSizeExceeded(msg) => write!(f, "File size exceeded: {}", msg),
            ImageVerificationError::ProcessingError(msg) => write!(f, "Image processing error: {}", msg),
            ImageVerificationError::ApiError(msg) => write!(f, "External API error: {}", msg),
            ImageVerificationError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ImageVerificationError::DuplicateImage(msg) => write!(f, "Duplicate image detected: {}", msg),
            ImageVerificationError::QualityInsufficient(msg) => write!(f, "Image quality insufficient: {}", msg),
        }
    }
}

impl Error for ImageVerificationError {}

// Image metadata structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub file_size: usize,
    pub aspect_ratio: f64,
    pub quality_score: f64,
    pub has_exif: bool,
    pub creation_timestamp: Option<String>,
    pub gps_coordinates: Option<(f64, f64)>,
    pub device_info: Option<String>,
}

// Image comparison result
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageComparisonResult {
    pub similarity_score: f64,
    pub structural_similarity: f64,
    pub color_similarity: f64,
    pub feature_matches: u32,
    pub is_likely_same_property: bool,
    pub confidence_level: f64,
    pub analysis_notes: Vec<String>,
}

// Batch verification result
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchVerificationResult {
    pub total_images: usize,
    pub valid_images: usize,
    pub rejected_images: usize,
    pub duplicates_found: usize,
    pub average_quality_score: f64,
    pub property_consistency_score: f64,
    pub verification_passed: bool,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

pub struct ImageVerificationService {
    client: Client,
    google_vision_api_key: Option<String>,
    max_file_size_mb: usize,
    min_dimensions: (u32, u32),
    supported_formats: Vec<String>,
}

impl ImageVerificationService {
    pub fn new(
        google_vision_api_key: Option<String>,
        max_file_size_mb: usize,
    ) -> Self {
        Self {
            client: Client::new(),
            google_vision_api_key,
            max_file_size_mb,
            min_dimensions: (800, 600), // Minimum image dimensions
            supported_formats: vec![
                "JPEG".to_string(),
                "PNG".to_string(),
                "WebP".to_string(),
                "HEIC".to_string(),
            ],
        }
    }

    /// Comprehensive image validation with metadata extraction
    pub async fn validate_and_analyze_image(
        &self,
        image_data: &[u8],
        filename: &str,
    ) -> Result<ImageMetadata, ImageVerificationError> {
        // 1. Basic size validation
        self.validate_file_size(image_data)?;

        // 2. Format validation and image loading
        let image_format = self.detect_image_format(image_data)?;
        let image = self.load_image(image_data)?;

        // 3. Dimension validation
        let (width, height) = image.dimensions();
        self.validate_dimensions(width, height)?;

        // 4. Quality assessment
        let quality_score = self.assess_image_quality(&image);

        // 5. Extract EXIF data if available
        let (has_exif, creation_timestamp, gps_coordinates, device_info) = 
            self.extract_exif_data(image_data).await;

        let metadata = ImageMetadata {
            width,
            height,
            format: format!("{:?}", image_format),
            file_size: image_data.len(),
            aspect_ratio: width as f64 / height as f64,
            quality_score,
            has_exif,
            creation_timestamp,
            gps_coordinates,
            device_info,
        };

        // 6. Validate minimum quality requirements
        if quality_score < 0.6 {
            return Err(ImageVerificationError::QualityInsufficient(
                format!("Image quality score ({:.2}) is below minimum requirement (0.60)", quality_score)
            ));
        }

        Ok(metadata)
    }

    /// Validate file size constraints
    fn validate_file_size(&self, image_data: &[u8]) -> Result<(), ImageVerificationError> {
        let max_size_bytes = self.max_file_size_mb * 1024 * 1024;
        if image_data.len() > max_size_bytes {
            return Err(ImageVerificationError::FileSizeExceeded(
                format!("File size ({} MB) exceeds maximum allowed ({} MB)", 
                    image_data.len() / (1024 * 1024), 
                    self.max_file_size_mb)
            ));
        }

        if image_data.len() < 1024 {
            return Err(ImageVerificationError::InvalidFormat(
                "File too small to be a valid image".to_string()
            ));
        }

        Ok(())
    }

    /// Detect and validate image format
    fn detect_image_format(&self, image_data: &[u8]) -> Result<ImageFormat, ImageVerificationError> {
        // Check file signatures (magic numbers)
        if image_data.len() < 12 {
            return Err(ImageVerificationError::InvalidFormat(
                "File too small to determine format".to_string()
            ));
        }

        let format = if image_data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            ImageFormat::Jpeg
        } else if image_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            ImageFormat::Png
        } else if image_data.len() >= 12 && 
                  &image_data[0..4] == b"RIFF" && 
                  &image_data[8..12] == b"WEBP" {
            ImageFormat::WebP
        } else if image_data.starts_with(&[0x00, 0x00, 0x00]) && 
                  image_data.len() > 8 &&
                  &image_data[4..8] == b"ftyp" {
            // HEIC format detection
            return Err(ImageVerificationError::InvalidFormat(
                "HEIC format not fully supported yet. Please convert to JPEG or PNG".to_string()
            ));
        } else {
            return Err(ImageVerificationError::InvalidFormat(
                "Unsupported image format. Only JPEG, PNG, and WebP are supported".to_string()
            ));
        };

        Ok(format)
    }

    /// Load and validate image data
    fn load_image(&self, image_data: &[u8]) -> Result<DynamicImage, ImageVerificationError> {
        image::load_from_memory(image_data)
            .map_err(|e| ImageVerificationError::ProcessingError(
                format!("Failed to load image: {}", e)
            ))
    }

    /// Validate image dimensions
    fn validate_dimensions(&self, width: u32, height: u32) -> Result<(), ImageVerificationError> {
        if width < self.min_dimensions.0 || height < self.min_dimensions.1 {
            return Err(ImageVerificationError::QualityInsufficient(
                format!("Image dimensions ({}x{}) are below minimum requirements ({}x{})",
                    width, height, self.min_dimensions.0, self.min_dimensions.1)
            ));
        }

        // Check for extremely wide or tall images (likely banners or invalid aspect ratios)
        let aspect_ratio = width as f64 / height as f64;
        if aspect_ratio > 3.0 || aspect_ratio < 0.33 {
            return Err(ImageVerificationError::QualityInsufficient(
                format!("Image aspect ratio ({:.2}) is outside acceptable range (0.33 - 3.0)", aspect_ratio)
            ));
        }

        Ok(())
    }

    /// Assess image quality using multiple metrics
    fn assess_image_quality(&self, image: &DynamicImage) -> f64 {
        let mut quality_score = 0.0;

        // 1. Resolution score (30%)
        let (width, height) = image.dimensions();
        let pixel_count = (width * height) as f64;
        let resolution_score = if pixel_count >= 2_000_000.0 { // 2MP+
            1.0
        } else if pixel_count >= 1_000_000.0 { // 1-2MP
            0.9
        } else if pixel_count >= 500_000.0 { // 0.5-1MP
            0.7
        } else {
            0.5
        };
        quality_score += resolution_score * 0.3;

        // 2. Sharpness assessment (25%)
        let sharpness_score = self.calculate_image_sharpness(image);
        quality_score += sharpness_score * 0.25;

        // 3. Brightness and contrast (20%)
        let (brightness_score, contrast_score) = self.analyze_brightness_contrast(image);
        quality_score += (brightness_score + contrast_score) / 2.0 * 0.2;

        // 4. Color richness (15%)
        let color_score = self.analyze_color_richness(image);
        quality_score += color_score * 0.15;

        // 5. Noise level (10%) - lower noise = higher score
        let noise_score = 1.0 - self.calculate_noise_level(image);
        quality_score += noise_score * 0.1;

        quality_score.min(1.0).max(0.0)
    }

    /// Calculate image sharpness using Laplacian variance
    fn calculate_image_sharpness(&self, image: &DynamicImage) -> f64 {
        let gray_image = image.to_luma8();
        let (width, height) = gray_image.dimensions();
        
        if width < 3 || height < 3 {
            return 0.5; // Cannot calculate for very small images
        }

        let mut laplacian_sum = 0.0;
        let mut pixel_count = 0;

        // Apply Laplacian kernel for edge detection
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let center = gray_image.get_pixel(x, y)[0] as f64;
                let top = gray_image.get_pixel(x, y - 1)[0] as f64;
                let bottom = gray_image.get_pixel(x, y + 1)[0] as f64;
                let left = gray_image.get_pixel(x - 1, y)[0] as f64;
                let right = gray_image.get_pixel(x + 1, y)[0] as f64;

                let laplacian = -4.0 * center + top + bottom + left + right;
                laplacian_sum += laplacian * laplacian;
                pixel_count += 1;
            }
        }

        let variance = laplacian_sum / pixel_count as f64;
        
        // Normalize sharpness score (typical range: 0-10000)
        let normalized_score = (variance / 1000.0).min(1.0);
        normalized_score
    }

    /// Analyze brightness and contrast
    fn analyze_brightness_contrast(&self, image: &DynamicImage) -> (f64, f64) {
        let gray_image = image.to_luma8();
        let pixels: Vec<u8> = gray_image.as_raw().clone();
        
        if pixels.is_empty() {
            return (0.5, 0.5);
        }

        // Calculate brightness (average pixel value)
        let sum: u64 = pixels.iter().map(|&p| p as u64).sum();
        let avg_brightness = sum as f64 / pixels.len() as f64;
        
        // Optimal brightness is around 128 (middle gray)
        let brightness_score = 1.0 - (avg_brightness - 128.0).abs() / 128.0;
        
        // Calculate contrast (standard deviation)
        let variance: f64 = pixels.iter()
            .map(|&p| {
                let diff = p as f64 - avg_brightness;
                diff * diff
            })
            .sum::<f64>() / pixels.len() as f64;
        
        let std_dev = variance.sqrt();
        
        // Good contrast should have std dev around 40-60
        let contrast_score = if std_dev < 20.0 {
            std_dev / 20.0 // Too low contrast
        } else if std_dev > 80.0 {
            1.0 - (std_dev - 80.0) / 80.0 // Too high contrast
        } else {
            1.0 // Good contrast
        };

        (brightness_score.max(0.0).min(1.0), contrast_score.max(0.0).min(1.0))
    }

    /// Analyze color richness and saturation
    fn analyze_color_richness(&self, image: &DynamicImage) -> f64 {
        let rgb_image = image.to_rgb8();
        let (width, height) = rgb_image.dimensions();
        let mut unique_colors = std::collections::HashSet::new();
        let mut total_saturation = 0.0;
        let mut pixel_count = 0;

        // Sample pixels to avoid performance issues on large images
        let step = std::cmp::max(1, (width * height) / 10000); // Sample ~10k pixels max

        for y in (0..height).step_by(step as usize) {
            for x in (0..width).step_by(step as usize) {
                let pixel = rgb_image.get_pixel(x, y);
                let r = pixel[0] as f64 / 255.0;
                let g = pixel[1] as f64 / 255.0;
                let b = pixel[2] as f64 / 255.0;

                // Add to unique colors (quantized to reduce memory usage)
                let quantized_color = (
                    (r * 16.0) as u8,
                    (g * 16.0) as u8,
                    (b * 16.0) as u8,
                );
                unique_colors.insert(quantized_color);

                // Calculate HSV saturation
                let max_val = r.max(g).max(b);
                let min_val = r.min(g).min(b);
                let saturation = if max_val > 0.0 {
                    (max_val - min_val) / max_val
                } else {
                    0.0
                };
                
                total_saturation += saturation;
                pixel_count += 1;
            }
        }

        // Color diversity score (0-1)
        let color_diversity = (unique_colors.len() as f64 / 4096.0).min(1.0);
        
        // Average saturation score (0-1)
        let avg_saturation = if pixel_count > 0 {
            total_saturation / pixel_count as f64
        } else {
            0.0
        };

        // Combined color richness score
        (color_diversity * 0.6 + avg_saturation * 0.4).min(1.0)
    }

    /// Calculate noise level in image
    fn calculate_noise_level(&self, image: &DynamicImage) -> f64 {
        let gray_image = image.to_luma8();
        let (width, height) = gray_image.dimensions();
        
        if width < 5 || height < 5 {
            return 0.1; // Assume low noise for very small images
        }

        let mut noise_sum = 0.0;
        let mut pixel_count = 0;

        // Calculate local variance to estimate noise
        for y in 2..(height - 2) {
            for x in 2..(width - 2) {
                let mut local_pixels = Vec::new();
                
                // Collect 5x5 neighborhood
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        let pixel_val = gray_image.get_pixel(
                            (x as i32 + dx) as u32, 
                            (y as i32 + dy) as u32
                        )[0] as f64;
                        local_pixels.push(pixel_val);
                    }
                }

                // Calculate local variance
                let mean = local_pixels.iter().sum::<f64>() / local_pixels.len() as f64;
                let variance = local_pixels.iter()
                    .map(|&p| (p - mean).powi(2))
                    .sum::<f64>() / local_pixels.len() as f64;

                noise_sum += variance;
                pixel_count += 1;
            }
        }

        let avg_noise = noise_sum / pixel_count as f64;
        
        // Normalize noise level (typical range: 0-100)
        (avg_noise / 100.0).min(1.0)
    }

    /// Extract EXIF metadata from image
    async fn extract_exif_data(&self, image_data: &[u8]) -> (bool, Option<String>, Option<(f64, f64)>, Option<String>) {
        // This is a simplified EXIF extraction
        // In production, you'd use a proper EXIF library like `exif` or `rexif`
        
        // Basic EXIF detection for JPEG images
        let has_exif = image_data.len() > 20 && 
                      image_data.starts_with(&[0xFF, 0xD8]) && // JPEG start
                      self.contains_exif_marker(image_data);

        // For now, return placeholder values
        // In production implementation, parse actual EXIF data
        let creation_timestamp = if has_exif {
            Some("2024-01-01T12:00:00Z".to_string()) // Placeholder
        } else {
            None
        };

        let gps_coordinates = if has_exif {
            // Extract GPS coordinates if available
            None // Placeholder - would extract real GPS data
        } else {
            None
        };

        let device_info = if has_exif {
            Some("Camera Model Unknown".to_string()) // Placeholder
        } else {
            None
        };

        (has_exif, creation_timestamp, gps_coordinates, device_info)
    }

    /// Check if image contains EXIF marker
    fn contains_exif_marker(&self, data: &[u8]) -> bool {
        // Look for EXIF marker in JPEG
        for i in 0..data.len().saturating_sub(4) {
            if &data[i..i+4] == b"Exif" {
                return true;
            }
        }
        false
    }

    /// Generate cryptographic hash of image content
    pub fn generate_image_hash(&self, image_data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(image_data);
        format!("{:x}", hasher.finalize())
    }

    /// Generate perceptual hash for duplicate detection
    pub fn generate_perceptual_hash(&self, image: &DynamicImage) -> Result<String, ImageVerificationError> {
        // Create 8x8 grayscale thumbnail
        let resized = image.resize_exact(8, 8, image::imageops::FilterType::Lanczos3);
        let gray = resized.to_luma8();
        
        // Calculate average pixel value
        let pixels: Vec<u8> = gray.as_raw().clone();
        let average: f64 = pixels.iter().map(|&p| p as f64).sum::<f64>() / 64.0;
        
        // Create hash based on whether each pixel is above or below average
        let mut hash = String::new();
        for &pixel in &pixels {
            hash.push(if pixel as f64 > average { '1' } else { '0' });
        }
        
        Ok(hash)
    }

    /// Compare two images for similarity
    pub async fn compare_images(
        &self,
        image1_data: &[u8],
        image2_data: &[u8],
    ) -> Result<ImageComparisonResult, ImageVerificationError> {
        // Load both images
        let image1 = self.load_image(image1_data)?;
        let image2 = self.load_image(image2_data)?;

        // Generate perceptual hashes
        let hash1 = self.generate_perceptual_hash(&image1)?;
        let hash2 = self.generate_perceptual_hash(&image2)?;

        // Calculate Hamming distance between perceptual hashes
        let hamming_distance = self.calculate_hamming_distance(&hash1, &hash2);
        let structural_similarity = 1.0 - (hamming_distance as f64 / 64.0);

        // Calculate color histogram similarity
        let color_similarity = self.calculate_color_histogram_similarity(&image1, &image2);

        // Overall similarity score (weighted average)
        let similarity_score = structural_similarity * 0.6 + color_similarity * 0.4;

        // Determine if images are likely of the same property
        let is_likely_same_property = similarity_score > 0.75;
        let confidence_level = if similarity_score > 0.9 {
            0.95
        } else if similarity_score > 0.8 {
            0.85
        } else if similarity_score > 0.7 {
            0.7
        } else {
            similarity_score
        };

        // Generate analysis notes
        let mut analysis_notes = Vec::new();
        
        if structural_similarity > 0.9 {
            analysis_notes.push("Very high structural similarity detected".to_string());
        } else if structural_similarity > 0.7 {
            analysis_notes.push("Good structural similarity".to_string());
        } else if structural_similarity < 0.3 {
            analysis_notes.push("Low structural similarity - likely different scenes".to_string());
        }

        if color_similarity > 0.8 {
            analysis_notes.push("Similar color palette detected".to_string());
        } else if color_similarity < 0.4 {
            analysis_notes.push("Different color characteristics".to_string());
        }

        Ok(ImageComparisonResult {
            similarity_score,
            structural_similarity,
            color_similarity,
            feature_matches: 0, // Placeholder - would use actual feature matching
            is_likely_same_property,
            confidence_level,
            analysis_notes,
        })
    }

    /// Calculate Hamming distance between two binary strings
    fn calculate_hamming_distance(&self, hash1: &str, hash2: &str) -> u32 {
        hash1.chars()
            .zip(hash2.chars())
            .map(|(c1, c2)| if c1 != c2 { 1 } else { 0 })
            .sum()
    }

    /// Calculate color histogram similarity
    fn calculate_color_histogram_similarity(&self, image1: &DynamicImage, image2: &DynamicImage) -> f64 {
        let hist1 = self.calculate_color_histogram(image1);
        let hist2 = self.calculate_color_histogram(image2);

        // Calculate histogram intersection
        let mut intersection = 0.0;
        let mut total1 = 0.0;
        let mut total2 = 0.0;

        for i in 0..256 {
            let val1 = hist1[i];
            let val2 = hist2[i];
            intersection += val1.min(val2);
            total1 += val1;
            total2 += val2;
        }

        if total1 > 0.0 && total2 > 0.0 {
            intersection / total1.max(total2)
        } else {
            0.0
        }
    }

    /// Calculate simple color histogram
    fn calculate_color_histogram(&self, image: &DynamicImage) -> Vec<f64> {
        let gray_image = image.to_luma8();
        let mut histogram = vec![0.0; 256];
        let pixels = gray_image.as_raw();

        for &pixel in pixels {
            histogram[pixel as usize] += 1.0;
        }

        histogram
    }

    /// Perform batch verification of multiple property images
    pub async fn verify_property_images_batch(
        &self,
        images: Vec<(&[u8], String)>, // (image_data, filename) pairs
        check_duplicates: bool,
    ) -> Result<BatchVerificationResult, ImageVerificationError> {
        let total_images = images.len();
        let mut valid_images = 0;
        let mut rejected_images = 0;
        let mut duplicates_found = 0;
        let mut quality_scores = Vec::new();
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        let mut image_hashes = std::collections::HashSet::new();

        for (image_data, filename) in images {
            match self.validate_and_analyze_image(image_data, &filename).await {
                Ok(metadata) => {
                    valid_images += 1;
                    quality_scores.push(metadata.quality_score);

                    // Check for duplicates if requested
                    if check_duplicates {
                        let content_hash = self.generate_image_hash(image_data);
                        if !image_hashes.insert(content_hash) {
                            duplicates_found += 1;
                            issues.push(format!("Duplicate image detected: {}", filename));
                        }
                    }

                    // Quality recommendations
                    if metadata.quality_score < 0.8 {
                        recommendations.push(format!("Consider improving quality of {}", filename));
                    }
                    
                    if metadata.width < 1200 || metadata.height < 900 {
                        recommendations.push(format!("Higher resolution recommended for {}", filename));
                    }
                },
                Err(e) => {
                    rejected_images += 1;
                    issues.push(format!("Image {} rejected: {}", filename, e));
                }
            }
        }

        let average_quality_score = if !quality_scores.is_empty() {
            quality_scores.iter().sum::<f64>() / quality_scores.len() as f64
        } else {
            0.0
        };

        // Calculate property consistency score
        let property_consistency_score = if valid_images >= 3 {
            0.9 // Assume high consistency if we have enough valid images
        } else if valid_images >= 1 {
            0.7
        } else {
            0.0
        };

        let verification_passed = valid_images >= 1 && 
                                 rejected_images as f64 / total_images as f64 <= 0.3 &&
                                 average_quality_score >= 0.6;

        // Add general recommendations
        if total_images < 3 {
            recommendations.push("Consider uploading at least 3-5 images for better property showcase".to_string());
        }
        
        if duplicates_found > 0 {
            recommendations.push("Remove duplicate images to improve listing quality".to_string());
        }

        Ok(BatchVerificationResult {
            total_images,
            valid_images,
            rejected_images,
            duplicates_found,
            average_quality_score,
            property_consistency_score,
            verification_passed,
            issues,
            recommendations,
        })
    }

    /// Detect if image appears to be AI-generated or heavily manipulated
    pub async fn detect_artificial_content(
        &self,
        image_data: &[u8],
    ) -> Result<f64, ImageVerificationError> {
        // This is a simplified implementation
        // In production, you'd use specialized AI detection models
        
        let image = self.load_image(image_data)?;
        let metadata = self.validate_and_analyze_image(image_data, "temp.jpg").await?;
        
        let mut artificial_score = 0.0;

        // Check for perfect symmetry (common in AI-generated images)
        let symmetry_score = self.calculate_symmetry_score(&image);
        if symmetry_score > 0.95 {
            artificial_score += 0.3;
        }

        // Check for unrealistic perfection in quality metrics
        if metadata.quality_score > 0.98 {
            artificial_score += 0.2;
        }

        // Check for missing EXIF data (many AI generators don't add realistic EXIF)
        if !metadata.has_exif {
            artificial_score += 0.1;
        }

        // Check for unusual noise patterns
        let noise_level = self.calculate_noise_level(&image);
        if noise_level < 0.01 { // Too perfect, potentially artificial
            artificial_score += 0.2;
        }

        artificial_score.min(1.0)
    }

    /// Calculate image symmetry score
    fn calculate_symmetry_score(&self, image: &DynamicImage) -> f64 {
        let gray_image = image.to_luma8();
        let (width, height) = gray_image.dimensions();
        
        if width < 10 || height < 10 {
            return 0.0;
        }

        let mut symmetry_sum = 0.0;
        let mut pixel_count = 0;

        // Check horizontal symmetry
        for y in 0..height {
            for x in 0..(width / 2) {
                let left_pixel = gray_image.get_pixel(x, y)[0];
                let right_pixel = gray_image.get_pixel(width - 1 - x, y)[0];
                
                let difference = (left_pixel as i32 - right_pixel as i32).abs() as f64;
                symmetry_sum += 1.0 - (difference / 255.0);
                pixel_count += 1;
            }
        }

        if pixel_count > 0 {
            symmetry_sum / pixel_count as f64
        } else {
            0.0
        }
    }
}