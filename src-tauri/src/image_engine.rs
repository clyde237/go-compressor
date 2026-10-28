//! Image compression engine using Rayon, adaptive scaling and dichotomy on quality Q.
//!
//! Output formats:
//! - JPEG and WebP: dichotomy on Q in [q_min, q_max], then adaptive downscaling
//! - PNG: lossless first, then 256-color palette quantization (NeuQuant), then downscaling

use color_quant::NeuQuant;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::metadata::Orientation;
use image::{DynamicImage, ImageEncoder, RgbImage, imageops};
use rayon::prelude::*;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
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

    #[error(
        "Impossible de calibrer l'image {0} sous la limite de 2 Mo même à Q=65 et échelle 0.40"
    )]
    CalibrationFailed(PathBuf),

    #[error("Quota d'images dépassé: maximum {0} photos par lot")]
    QuotaExceeded(usize),
}

/// Format d'image produit par le moteur
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageOutputFormat {
    /// Conserve le format source s'il est encodable (JPEG, PNG, WebP), sinon JPEG
    Auto,
    #[default]
    Jpeg,
    Webp,
    Png,
}

impl ImageOutputFormat {
    /// Format concret encodable correspondant à l'extension source, s'il existe
    pub fn from_source(input_path: &Path) -> Option<Self> {
        let ext = input_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "webp" => Some(Self::Webp),
            _ => None,
        }
    }

    /// Résout le format concret (jamais `Auto`) et l'extension du fichier produit
    pub fn resolve(self, input_path: &Path) -> (Self, String) {
        let concrete = match self {
            Self::Auto => Self::from_source(input_path).unwrap_or(Self::Jpeg),
            other => other,
        };
        let source_ext = input_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let ext = match concrete {
            // Conserve la graphie .jpeg d'origine quand le format ne change pas
            Self::Jpeg if source_ext == "jpeg" => "jpeg",
            Self::Jpeg | Self::Auto => "jpg",
            Self::Webp => "webp",
            Self::Png => "png",
        };
        (concrete, ext.to_string())
    }
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
    pub output_format: ImageOutputFormat,
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
            output_format: ImageOutputFormat::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub output_path: PathBuf,
    pub format: ImageOutputFormat,
    /// Facteur Q retenu (JPEG/WebP) ; 100 = PNG sans perte, 0 = PNG palette 256 couleurs
    pub quality: u8,
    pub scale: f32,
    pub original_size_bytes: usize,
    pub final_size_bytes: usize,
    pub gain_percent: u32,
}

/// Convertit en RGB en aplatissant la transparence sur fond blanc (JPEG ne gère pas l'alpha)
fn to_rgb_on_white(img: &DynamicImage) -> RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let blend = |c: u8| ((c as u16 * a as u16 + 255 * (255 - a as u16)) / 255) as u8;
        image::Rgb([blend(r), blend(g), blend(b)])
    })
}

/// Encode une DynamicImage en JPEG avec le facteur de qualité Q spécifié
pub fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, image::ImageError> {
    let rgb = to_rgb_on_white(img);
    let mut out = Vec::new();
    let mut cursor = Cursor::new(&mut out);
    let mut encoder = JpegEncoder::new_with_quality(&mut cursor, quality);
    encoder.encode_image(&rgb)?;
    Ok(out)
}

/// Encode une DynamicImage en WebP avec perte (libwebp) au facteur de qualité Q
pub fn encode_webp(img: &DynamicImage, quality: u8) -> Vec<u8> {
    if img.color().has_alpha() {
        let rgba = img.to_rgba8();
        webp::Encoder::from_rgba(&rgba, rgba.width(), rgba.height())
            .encode(quality as f32)
            .to_vec()
    } else {
        let rgb = img.to_rgb8();
        webp::Encoder::from_rgb(&rgb, rgb.width(), rgb.height())
            .encode(quality as f32)
            .to_vec()
    }
}

/// Encode une DynamicImage en PNG sans perte, compression DEFLATE maximale
pub fn encode_png_lossless(img: &DynamicImage) -> Result<Vec<u8>, image::ImageError> {
    let mut out = Vec::new();
    let encoder = PngEncoder::new_with_quality(
        Cursor::new(&mut out),
        CompressionType::Best,
        PngFilterType::Adaptive,
    );
    if img.color().has_alpha() {
        let rgba = img.to_rgba8();
        encoder.write_image(
            &rgba,
            rgba.width(),
            rgba.height(),
            image::ExtendedColorType::Rgba8,
        )?;
    } else {
        let rgb = img.to_rgb8();
        encoder.write_image(
            &rgb,
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )?;
    }
    Ok(out)
}

/// Encode une DynamicImage en PNG indexé (palette de 256 couleurs quantifiée par NeuQuant).
/// La transparence est conservée via le segment tRNS.
pub fn encode_png_quantized(img: &DynamicImage) -> Result<Vec<u8>, png::EncodingError> {
    let rgba = img.to_rgba8();
    let quantizer = NeuQuant::new(10, 256, &rgba);
    let indices: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|px| quantizer.index_of(px) as u8)
        .collect();

    let color_map = quantizer.color_map_rgba();
    let (entries, _) = color_map.as_chunks::<4>();
    let palette: Vec<u8> = entries.iter().flat_map(|c| [c[0], c[1], c[2]]).collect();
    let alphas: Vec<u8> = entries.iter().map(|c| c[3]).collect();

    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, rgba.width(), rgba.height());
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_palette(palette);
    if alphas.iter().any(|&a| a < 255) {
        encoder.set_trns(alphas);
    }
    encoder.set_compression(png::Compression::High);
    // Les images à palette se compressent mieux sans filtre de prédiction
    encoder.set_filter(png::Filter::NoFilter);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&indices)?;
    writer.finish()?;
    Ok(out)
}

/// Redimensionnement adaptatif avec filtre Lanczos3 haute fidélité
pub fn resize_adaptive(img: &DynamicImage, scale: f32) -> DynamicImage {
    if (scale - 1.0).abs() < f32::EPSILON {
        return img.clone();
    }
    let width = ((img.width() as f32) * scale).round() as u32;
    let height = ((img.height() as f32) * scale).round() as u32;
    img.resize_exact(width.max(1), height.max(1), imageops::FilterType::Lanczos3)
}

/// Recherche dichotomique du facteur de qualité Q dans [q_min, q_max] (encodage JPEG)
/// Privilégie la plus haute qualité dans la fourchette cible [size_min, size_max]
pub fn binary_search_quality(
    img: &DynamicImage,
    original_size: usize,
    q_min: u8,
    q_max: u8,
    size_min: usize,
    size_max: usize,
) -> Option<(u8, Vec<u8>)> {
    binary_search_quality_with(
        img,
        original_size,
        q_min,
        q_max,
        size_min,
        size_max,
        |i, q| encode_jpeg(i, q).ok(),
    )
}

/// Recherche dichotomique du facteur Q avec un encodeur avec perte quelconque (JPEG, WebP)
pub fn binary_search_quality_with<E>(
    img: &DynamicImage,
    original_size: usize,
    q_min: u8,
    q_max: u8,
    size_min: usize,
    size_max: usize,
    encode: E,
) -> Option<(u8, Vec<u8>)>
where
    E: Fn(&DynamicImage, u8) -> Option<Vec<u8>>,
{
    let mut low = q_min;
    let mut high = q_max;
    let mut best: Option<(u8, Vec<u8>)> = None;

    while low <= high {
        let mid = (low + high) / 2;
        let bytes = encode(img, mid)?;
        let size = bytes.len();

        if size > size_max {
            high = mid.saturating_sub(1);
        } else if size < size_min {
            let is_better = best
                .as_ref()
                .map(|(_, prev)| size > prev.len())
                .unwrap_or(true);
            if is_better {
                best = Some((mid, bytes));
            }
            low = mid + 1;
        } else {
            let is_better = best
                .as_ref()
                .map(|(_, prev)| size > prev.len())
                .unwrap_or(true);
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

/// Lit le tag EXIF Orientation (1 à 8) du conteneur source
fn read_exif_orientation(bytes: &[u8]) -> Option<Orientation> {
    let exif = exif::Reader::new()
        .read_from_container(&mut Cursor::new(bytes))
        .ok()?;
    let value = exif
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)?
        .value
        .get_uint(0)?;
    Orientation::from_exif(u8::try_from(value).ok()?)
}

/// Décode une image depuis les octets en mémoire avec fallback FFmpeg pour les formats exotiques (HEIC, AVIF, SVG, RAW...)
pub fn load_image_with_fallback(
    bytes: &[u8],
    path: &Path,
) -> Result<DynamicImage, ImageEngineError> {
    decode_image(bytes, path).map(|(img, _)| img)
}

/// Décode l'image ; le booléen indique si le décodage natif (crate image) a réussi
fn decode_image(bytes: &[u8], path: &Path) -> Result<(DynamicImage, bool), ImageEngineError> {
    if let Ok(img) = image::load_from_memory(bytes) {
        return Ok((img, true));
    }

    // Fallback FFmpeg : décoder une image complexe (HEIC, AVIF, RAW, SVG, etc.) en PNG standard via un tube mémoire
    let ffmpeg = crate::video_engine::resolve_ffmpeg_path();
    let maybe_img = std::process::Command::new(&ffmpeg)
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(path)
        .args(["-frames:v", "1", "-f", "image2pipe", "-vcodec", "png", "-"])
        .output()
        .ok()
        .filter(|o| o.status.success() && !o.stdout.is_empty())
        .and_then(|o| image::load_from_memory(&o.stdout).ok());

    if let Some(img) = maybe_img {
        return Ok((img, false));
    }

    image::load_from_memory(bytes)
        .map(|img| (img, true))
        .map_err(|e| ImageEngineError::ImageDecodeError(path.to_path_buf(), e))
}

/// Ordre de recherche préconisé : commence par scale = 1.0 afin de préserver au maximum la résolution
const SCALES: [f32; 6] = [1.0, 0.85, 0.70, 0.60, 0.50, 0.40];

/// Recherche (Q, échelle) pour un format avec perte : premier couple qui tient dans la cible
fn search_lossy(
    img: &DynamicImage,
    format: ImageOutputFormat,
    original_size: usize,
    options: &ImageCompressionOptions,
) -> Option<(u8, f32, Vec<u8>)> {
    SCALES.iter().find_map(|&scale| {
        let resized = resize_adaptive(img, scale);
        binary_search_quality_with(
            &resized,
            original_size,
            options.q_min,
            options.q_max,
            options.target_min_bytes as usize,
            options.target_max_bytes as usize,
            |i, q| match format {
                ImageOutputFormat::Webp => Some(encode_webp(i, q)),
                _ => encode_jpeg(i, q).ok(),
            },
        )
        .map(|(q, bytes)| (q, scale, bytes))
    })
}

/// Recherche PNG : sans perte à pleine résolution si cela tient sous `size_max`,
/// sinon palette 256 couleurs avec réduction d'échelle progressive.
/// PNG n'ayant pas de facteur Q, la plus petite version est retenue si aucune ne tient.
fn search_png(img: &DynamicImage, size_max: usize) -> Option<(u8, f32, Vec<u8>)> {
    let lossless = encode_png_lossless(img).ok();
    if let Some(bytes) = lossless.as_ref().filter(|b| b.len() <= size_max) {
        return Some((100, 1.0, bytes.clone()));
    }

    let mut smallest: Option<(u8, f32, Vec<u8>)> = lossless.map(|b| (100, 1.0, b));
    for &scale in &SCALES {
        let resized = resize_adaptive(img, scale);
        let Ok(bytes) = encode_png_quantized(&resized) else {
            continue;
        };
        if bytes.len() <= size_max {
            return Some((0, scale, bytes));
        }
        if smallest
            .as_ref()
            .is_none_or(|(_, _, s)| bytes.len() < s.len())
        {
            smallest = Some((0, scale, bytes));
        }
    }
    smallest
}

/// Compresse une image unique en appliquant l'algorithme adaptatif multi-échelle et dichotomique
pub fn compress_single_image(
    input_path: &Path,
    output_path: &Path,
    options: &ImageCompressionOptions,
) -> Result<CompressionResult, ImageEngineError> {
    let original_bytes =
        fs::read(input_path).map_err(|e| ImageEngineError::IoError(input_path.to_path_buf(), e))?;
    let original_size = original_bytes.len();

    let (mut img, decoded_natively) = decode_image(&original_bytes, input_path)?;
    let (format, _) = options.output_format.resolve(input_path);

    // Le bloc EXIF n'est réinjecté que dans un JPEG. Sans lui, l'orientation est
    // appliquée aux pixels pour que les photos de smartphone restent droites.
    // (Le repli FFmpeg applique déjà la rotation : ne pas la doubler.)
    let exif = if options.preserve_exif && format == ImageOutputFormat::Jpeg {
        extract_exif(&original_bytes)
    } else {
        None
    };
    if exif.is_none()
        && decoded_natively
        && let Some(orientation) = read_exif_orientation(&original_bytes)
    {
        img.apply_orientation(orientation);
    }

    let (quality, scale, mut bytes) = match format {
        ImageOutputFormat::Png => search_png(&img, options.target_max_bytes as usize),
        _ => search_lossy(&img, format, original_size, options),
    }
    .ok_or_else(|| ImageEngineError::CalibrationFailed(input_path.to_path_buf()))?;

    if let Some(ex) = exif.as_ref() {
        bytes = inject_exif(bytes, ex);
    }

    // Garde-fou : à format identique, ne jamais livrer un fichier plus lourd que l'original
    if ImageOutputFormat::from_source(input_path) == Some(format) && bytes.len() >= original_size {
        bytes = original_bytes;
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

    Ok(CompressionResult {
        output_path: output_path.to_path_buf(),
        format,
        quality,
        scale,
        original_size_bytes: original_size,
        final_size_bytes: final_size,
        gain_percent,
    })
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
    let num_threads = if options.threads > 0 {
        options.threads
    } else {
        4
    };
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

                let (_, output_ext) = options.output_format.resolve(path);
                let dest_path = crate::export_engine::resolve_export_path(
                    path,
                    output_dir,
                    export_mode,
                    folder_timestamp,
                    Some(&output_ext),
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
            output_format: ImageOutputFormat::Jpeg,
        };

        let res =
            compress_single_image(&in_path, &out_path, &options).expect("compression reussie");
        assert!(res.final_size_bytes > 0);
        assert!(out_path.exists());

        let _ = fs::remove_file(in_path);
        let _ = fs::remove_file(out_path);
        let _ = fs::remove_dir(temp_dir);
    }

    #[test]
    fn test_compress_single_image_png_input() {
        let temp_dir = std::env::temp_dir().join("go_compressor_format_test");
        let _ = fs::create_dir_all(&temp_dir);
        let in_png = temp_dir.join("test_in.png");
        let out_jpg = temp_dir.join("test_out.jpg");

        let img = create_test_image(300, 300);
        img.save_with_format(&in_png, image::ImageFormat::Png)
            .expect("save png");

        let options = ImageCompressionOptions {
            target_min_bytes: 1_000,
            target_max_bytes: 500_000,
            target_bytes: 50_000,
            q_min: 65,
            q_max: 92,
            threads: 2,
            preserve_exif: false,
            output_format: ImageOutputFormat::Jpeg,
        };

        let res =
            compress_single_image(&in_png, &out_jpg, &options).expect("compression png reussie");
        assert!(res.final_size_bytes > 0);
        assert!(out_jpg.exists());

        let _ = fs::remove_file(in_png);
        let _ = fs::remove_file(out_jpg);
        let _ = fs::remove_dir(temp_dir);
    }
    fn compress_to(
        name: &str,
        img: &DynamicImage,
        source_format: image::ImageFormat,
        output_format: ImageOutputFormat,
    ) -> (CompressionResult, Vec<u8>) {
        let temp_dir = std::env::temp_dir().join(format!("go_compressor_fmt_{name}"));
        let _ = fs::create_dir_all(&temp_dir);
        let ext = source_format.extensions_str()[0];
        let in_path = temp_dir.join(format!("source.{ext}"));
        img.save_with_format(&in_path, source_format)
            .expect("écriture source");
        let (_, out_ext) = output_format.resolve(&in_path);
        let out_path = temp_dir.join(format!("sortie.{out_ext}"));

        let options = ImageCompressionOptions {
            target_min_bytes: 1_000,
            target_max_bytes: 500_000,
            output_format,
            preserve_exif: false,
            ..Default::default()
        };
        let res = compress_single_image(&in_path, &out_path, &options).expect("compression");
        let bytes = fs::read(&out_path).expect("lecture sortie");
        let _ = fs::remove_dir_all(&temp_dir);
        (res, bytes)
    }

    #[test]
    fn test_output_format_resolution() {
        let png = Path::new("/a/capture.png");
        let heic = Path::new("/a/iphone.heic");
        let jpeg = Path::new("/a/photo.jpeg");
        assert_eq!(
            ImageOutputFormat::Auto.resolve(png),
            (ImageOutputFormat::Png, "png".into())
        );
        assert_eq!(
            ImageOutputFormat::Auto.resolve(heic),
            (ImageOutputFormat::Jpeg, "jpg".into())
        );
        assert_eq!(
            ImageOutputFormat::Jpeg.resolve(jpeg),
            (ImageOutputFormat::Jpeg, "jpeg".into())
        );
        assert_eq!(
            ImageOutputFormat::Webp.resolve(png),
            (ImageOutputFormat::Webp, "webp".into())
        );
    }

    #[test]
    fn test_png_source_to_webp_produces_real_webp() {
        let img = create_test_image(300, 300);
        let (res, bytes) = compress_to(
            "webp",
            &img,
            image::ImageFormat::Png,
            ImageOutputFormat::Webp,
        );
        assert_eq!(res.format, ImageOutputFormat::Webp);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
        let decoded = image::load_from_memory(&bytes).expect("WebP décodable");
        assert_eq!((decoded.width(), decoded.height()), (300, 300));
    }

    #[test]
    fn test_jpeg_output_contains_real_jpeg_bytes() {
        let img = create_test_image(300, 300);
        let (_, bytes) = compress_to(
            "jpeg",
            &img,
            image::ImageFormat::Png,
            ImageOutputFormat::Jpeg,
        );
        assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn test_png_output_keeps_transparency() {
        let mut rgba = image::RgbaImage::new(200, 200);
        for (x, y, px) in rgba.enumerate_pixels_mut() {
            *px = image::Rgba([
                (x % 256) as u8,
                (y % 256) as u8,
                120,
                if x < 100 { 0 } else { 255 },
            ]);
        }
        let img = DynamicImage::ImageRgba8(rgba);
        let (res, bytes) =
            compress_to("png", &img, image::ImageFormat::Png, ImageOutputFormat::Png);
        assert_eq!(res.format, ImageOutputFormat::Png);
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory(&bytes)
            .expect("PNG décodable")
            .to_rgba8();
        assert_eq!(
            decoded.get_pixel(10, 10).0[3],
            0,
            "zone transparente perdue"
        );
        assert_eq!(decoded.get_pixel(150, 10).0[3], 255);
    }

    #[test]
    fn test_png_quantization_reduces_size() {
        // Image « photographique » : dégradé + bruit, des milliers de couleurs
        let mut seed: u32 = 42;
        let rgb = RgbImage::from_fn(400, 400, |x, y| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let n = ((seed >> 16) % 24) as u8;
            Rgb([
                (x / 2) as u8 + n,
                (y / 2) as u8 + n,
                ((x + y) / 4) as u8 + n,
            ])
        });
        let img = DynamicImage::ImageRgb8(rgb);
        let lossless = encode_png_lossless(&img).expect("png sans perte");
        let quantized = encode_png_quantized(&img).expect("png palette");
        assert!(quantized.len() < lossless.len());
    }

    #[test]
    fn test_same_format_output_never_heavier_than_source() {
        // Petit JPEG très compressé : le réencodage à Q élevé serait plus lourd
        let temp_dir = std::env::temp_dir().join("go_compressor_never_heavier");
        let _ = fs::create_dir_all(&temp_dir);
        let in_path = temp_dir.join("petit.jpg");
        let out_path = temp_dir.join("petit_opt.jpg");
        let source = encode_jpeg(&create_test_image(200, 200), 30).expect("encode");
        fs::write(&in_path, &source).expect("write");

        let options = ImageCompressionOptions {
            target_min_bytes: 1_000,
            target_max_bytes: 500_000,
            preserve_exif: false,
            ..Default::default()
        };
        let res = compress_single_image(&in_path, &out_path, &options).expect("compression");
        assert!(res.final_size_bytes <= source.len());
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
