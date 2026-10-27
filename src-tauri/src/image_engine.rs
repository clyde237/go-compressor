//! Image compression engine using Rayon and image codecs.

pub const TARGET_MIN_BYTES: u64 = 800 * 1024; // 800 Ko (819 200 octets)
pub const TARGET_MAX_BYTES: u64 = 2 * 1024 * 1024; // 2 Mo (2 097 152 octets)
pub const MAX_IMAGES: usize = 50;

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
            target_bytes: 1_104 * 1024, // 1104 Ko par défaut (~1.1 Mo)
            q_min: 65,
            q_max: 92,
            threads: 4,
            preserve_exif: true,
        }
    }
}

pub fn is_image_quota_valid(count: usize) -> bool {
    count <= MAX_IMAGES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options_within_bounds() {
        let opts = ImageCompressionOptions::default();
        assert_eq!(opts.q_min, 65);
        assert_eq!(opts.q_max, 92);
        assert!(opts.target_bytes >= opts.target_min_bytes);
        assert!(opts.target_bytes <= opts.target_max_bytes);
        assert_eq!(opts.target_min_bytes, 800 * 1024);
        assert_eq!(opts.target_max_bytes, 2 * 1024 * 1024);
    }

    #[test]
    fn test_image_quota_validation() {
        assert!(is_image_quota_valid(0));
        assert!(is_image_quota_valid(50));
        assert!(!is_image_quota_valid(51));
    }
}
