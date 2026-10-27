//! Image compression engine using Rayon, adaptive scaling and dichotomy on quality Q.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use image::codecs::jpeg::JpegEncoder;
use image::{imageops, DynamicImage};
use rayon::prelude::*;
use thiserror::Error;

pub const TARGET_MIN_BYTES: u64 = 800 * 1024; // 800 Ko (819 200 octets)
pub const TARGET_MAX_BYTES: u64 = 2_000 * 1024; // 2 Mo (2 048 000 octets)
pub const MAX_IMAGES: usize = 50;

pub const DEFAULT_Q_MIN: u8 = 65;
pub const DEFAULT_Q_MAX: u8 = 92;

pub const GAIN_MIN: f64 = 0.05; // 95 % de réduction (S_finale <= 0.15 * S0)
pub const GAIN_MAX: f64 = 0.15; // 85 % de réduction (S_finale >= 0.05 * S0)

#[derive(Error, Debug)]
pub enum ImageEngineError {
    #[error("Erreur d'entrée/sortie sur le fichier {0}: {1}")]
    IoError(PathBuf, #[source] std::io::Error),

    #[error("Format d'image non supporté ou fichier corrompu: {0}")]
    ImageDecodeError(PathBuf, #[source] image::ImageError),

    #[error("Impossible de calibrer l'image {0} sous la limite de 2 Mo même à Q=65 et échelle 0.40")]
    CalibrationFailed(PathBuf),

    #[error("Quota d'images dépassé: maximum {0} photos par lot")]
    QuotaExceeded(usize),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ImageCompressionOptions {
    pub target_min_bytes: u64,
    pub target_max_bytes: u64,
    pub target_bytes: u64,
    pub q_min: u8,
    pub q_max: u8,
    pub threads: usize,
    pub preserve_exif: bool,
}

impl Default for ImageCompressionOptions {
    fn default() -> Self {
        Self {
            target_min_bytes: TARGET_MIN_BYTES,
            target_max_bytes: TARGET_MAX_BYTES,
            target_bytes: 1_104 * 1024,
            q_min: DEFAULT_Q_MIN,
            q_max: DEFAULT_Q_MAX,
            threads: 4,
            preserve_exif: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub output_path: PathBuf,
    pub quality: u8,
    pub scale: f32,
    pub original_size_bytes: usize,
    pub final_size_bytes: usize,
    pub gain_percent: u32,
}

/// Encode une DynamicImage en JPEG avec le facteur de qualité Q spécifié
pub fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, image::ImageError> {
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    let mut cursor = Cursor::new(&mut out);
    let mut encoder = JpegEncoder::new_with_quality(&mut cursor, quality);
    encoder.encode_image(&rgb)?;
    Ok(out)
}

/// Redimensionnement adaptatif avec filtre Lanczos3 haute fidélité
pub fn resize_adaptive(img: &DynamicImage, scale: f32) -> DynamicImage {
    if (scale - 1.0).abs() < f32::EPSILON {
        return img.clone();
    }
    let width = ((img.width() as f32) * scale).round() as u32;
    let height = ((img.height() as f32) * scale).round() as u32;
    img.resize_exact(
        width.max(1),
        height.max(1),
        imageops::FilterType::Lanczos3,
    )
}

/// Recherche dichotomique du facteur de qualité Q dans [q_min, q_max]
/// Privilégie la plus haute qualité dans la fourchette cible [size_min, size_max]
pub fn binary_search_quality(
    img: &DynamicImage,
    original_size: usize,
    q_min: u8,
    q_max: u8,
    size_min: usize,
    size_max: usize,
) -> Option<(u8, Vec<u8>)> {
    let mut low = q_min;
    let mut high = q_max;
    let mut best: Option<(u8, Vec<u8>)> = None;

    while low <= high {
        let mid = (low + high) / 2;
        let bytes = encode_jpeg(img, mid).ok()?;
        let size = bytes.len();

        if size > size_max {
            high = mid.saturating_sub(1);
        } else if size < size_min {
            let is_better = best.as_ref().map(|(_, prev)| size > prev.len()).unwrap_or(true);
            if is_better {
                best = Some((mid, bytes));
            }
            low = mid + 1;
        } else {
            let is_better = best.as_ref().map(|(_, prev)| size > prev.len()).unwrap_or(true);
            if is_better {
                best = Some((mid, bytes));
            }
            low = mid + 1;
        }
    }

    if let Some((_, ref bytes)) = best {
        let original_safe = original_size.max(1);
        let ratio = bytes.len() as f64 / original_safe as f64;
        if (GAIN_MIN..=GAIN_MAX).contains(&ratio) {
            return best;
        }
    }

    best
}

/// Extrait les métadonnées EXIF brutes du conteneur d'image
pub fn extract_exif(bytes: &[u8]) -> Option<Vec<u8>> {
    let reader = exif::Reader::new();
    let mut cursor = Cursor::new(bytes);
    let exif = reader.read_from_container(&mut cursor).ok()?;
    Some(exif.buf().to_vec())
}

/// Réinjecte le segment APP1 EXIF dans le flux JPEG compressé (juste après le marqueur SOI 0xFFD8)
pub fn inject_exif(jpeg: Vec<u8>, exif: &[u8]) -> Vec<u8> {
    if jpeg.len() < 2 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return jpeg;
    }

    let exif_data = if exif.starts_with(b"Exif\0\0") {
        exif.to_vec()
    } else {
        let mut with_header = Vec::with_capacity(6 + exif.len());
        with_header.extend_from_slice(b"Exif\0\0");
        with_header.extend_from_slice(exif);
        with_header
    };

    let app1_len = (exif_data.len() + 2) as u16;
    let mut result = Vec::with_capacity(jpeg.len() + exif_data.len() + 4);
    result.extend_from_slice(&jpeg[..2]); // SOI
    result.extend_from_slice(&[0xFF, 0xE1]); // APP1
    result.extend_from_slice(&app1_len.to_be_bytes());
    result.extend_from_slice(&exif_data);
    result.extend_from_slice(&jpeg[2..]);
    result
}

/// Compresse une image unique en appliquant l'algorithme adaptatif multi-échelle et dichotomique
pub fn compress_single_image(
    input_path: &Path,
    output_path: &Path,
    options: &ImageCompressionOptions,
) -> Result<CompressionResult, ImageEngineError> {
    let original_bytes = fs::read(input_path)
        .map_err(|e| ImageEngineError::IoError(input_path.to_path_buf(), e))?;
    let original_size = original_bytes.len();

    let img = image::load_from_memory(&original_bytes)
        .map_err(|e| ImageEngineError::ImageDecodeError(input_path.to_path_buf(), e))?;

    let exif = if options.preserve_exif {
        extract_exif(&original_bytes)
    } else {
        None
    };

    // Ordre de recherche préconisé : commence par scale = 1.0 afin de préserver au maximum la résolution
    let scales = [1.0_f32, 0.85, 0.70, 0.60, 0.50, 0.40];

    for &scale in &scales {
        let resized = resize_adaptive(&img, scale);
        if let Some((quality, mut bytes)) = binary_search_quality(
            &resized,
            original_size,
            options.q_min,
            options.q_max,
            options.target_min_bytes as usize,
            options.target_max_bytes as usize,
        ) {
            if let Some(ex) = exif.as_ref().filter(|_| options.preserve_exif) {
                bytes = inject_exif(bytes, ex);
            }

            if let Some(parent) = output_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            fs::write(output_path, &bytes)
                .map_err(|e| ImageEngineError::IoError(output_path.to_path_buf(), e))?;

            let final_size = bytes.len();
            let gain_percent = if original_size > 0 && final_size < original_size {
                (((original_size - final_size) as f64 / original_size as f64) * 100.0) as u32
            } else {
                0
            };

            return Ok(CompressionResult {
                output_path: output_path.to_path_buf(),
                quality,
                scale,
                original_size_bytes: original_size,
                final_size_bytes: final_size,
                gain_percent,
            });
        }
    }

    Err(ImageEngineError::CalibrationFailed(input_path.to_path_buf()))
}

/// Traitement parallélisé d'un lot d'images avec Rayon
pub fn compress_images_batch<F>(
    images: &[String],
    output_dir: &Path,
    export_mode: &crate::export_engine::ExportMode,
    folder_timestamp: Option<&str>,
    options: &ImageCompressionOptions,
    progress_callback: F,
) -> Vec<crate::ProcessedMediaItem>
where
    F: Fn(String, f32, u64, &str, bool, bool, Option<String>) + Sync + Send,
{
    let num_threads = if options.threads > 0 { options.threads } else { 4 };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

    pool.install(|| {
        images
            .par_iter()
            .map(|img_path_str| {
                let path = Path::new(img_path_str);
                let orig_len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                progress_callback(
                    img_path_str.clone(),
                    10.0,
                    0,
                    "Début optimisation...",
                    false,
                    false,
                    None,
                );

                let dest_path = crate::export_engine::resolve_export_path(
                    path,
                    output_dir,
                    export_mode,
                    folder_timestamp,
                );

                match compress_single_image(path, &dest_path, options) {
                    Ok(res) => {
                        progress_callback(
                            img_path_str.clone(),
                            100.0,
                            res.final_size_bytes as u64,
                            "Optimisation terminée",
                            true,
                            false,
                            None,
                        );
                        crate::ProcessedMediaItem {
                            file_path: img_path_str.clone(),
                            original_size_bytes: orig_len,
                            output_size_bytes: res.final_size_bytes as u64,
                            output_path: dest_path.to_string_lossy().to_string(),
                            gain_percent: res.gain_percent,
                            success: true,
                            error_message: None,
                        }
                    }
                    Err(err) => {
                        progress_callback(
                            img_path_str.clone(),
                            100.0,
                            0,
                            "Échec d'optimisation",
                            true,
                            true,
                            Some(err.to_string()),
                        );
                        crate::ProcessedMediaItem {
                            file_path: img_path_str.clone(),
                            original_size_bytes: orig_len,
                            output_size_bytes: 0,
                            output_path: String::new(),
                            gain_percent: 0,
                            success: false,
                            error_message: Some(err.to_string()),
                        }
                    }
                }
            })
            .collect()
    })
}

pub fn is_image_quota_valid(count: usize) -> bool {
    count <= MAX_IMAGES
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        let mut img = RgbImage::new(width, height);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8]);
        }
        DynamicImage::ImageRgb8(img)
    }

    #[test]
    fn test_encode_jpeg_quality() {
        let img = create_test_image(200, 200);
        let q70 = encode_jpeg(&img, 70).expect("encodage Q70");
        let q90 = encode_jpeg(&img, 90).expect("encodage Q90");
        assert!(q90.len() > q70.len());
    }

    #[test]
    fn test_resize_adaptive() {
        let img = create_test_image(100, 100);
        let scaled = resize_adaptive(&img, 0.5);
        assert_eq!(scaled.width(), 50);
        assert_eq!(scaled.height(), 50);
    }

    #[test]
    fn test_binary_search_quality() {
        let img = create_test_image(800, 800);
        let result = binary_search_quality(&img, 500_000, 65, 92, 10_000, 200_000);
        assert!(result.is_some());
        let (q, bytes) = result.unwrap();
        assert!((65..=92).contains(&q));
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_exif_injection() {
        let fake_jpeg = vec![0xFF, 0xD8, 0xFF, 0xD9];
        let fake_exif = b"Exif\0\0fake_metadata";
        let injected = inject_exif(fake_jpeg, fake_exif);
        assert_eq!(&injected[..2], &[0xFF, 0xD8]);
        assert_eq!(&injected[2..4], &[0xFF, 0xE1]);
    }

    #[test]
    fn test_compress_single_image_with_synthetic_file() {
        let temp_dir = std::env::temp_dir().join("go_compressor_test");
        let _ = fs::create_dir_all(&temp_dir);
        let in_path = temp_dir.join("test_in.jpg");
        let out_path = temp_dir.join("test_out.jpg");

        let img = create_test_image(400, 400);
        let raw_jpeg = encode_jpeg(&img, 85).expect("encode initial");
        fs::write(&in_path, &raw_jpeg).expect("write temp in");

        let options = ImageCompressionOptions {
            target_min_bytes: 1_000,
            target_max_bytes: 500_000,
            target_bytes: 50_000,
            q_min: 65,
            q_max: 92,
            threads: 2,
            preserve_exif: false,
        };

        let res = compress_single_image(&in_path, &out_path, &options).expect("compression reussie");
        assert!(res.final_size_bytes > 0);
        assert!(out_path.exists());

        let _ = fs::remove_file(in_path);
        let _ = fs::remove_file(out_path);
        let _ = fs::remove_dir(temp_dir);
    }
}
