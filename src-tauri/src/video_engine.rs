//! Video transcoding engine using FFmpeg sidecar and sequential Tokio queue.

pub const MAX_VIDEOS: usize = 20;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct VideoTranscodeOptions {
    pub crf: u8,
    pub max_videos: usize,
}

impl Default for VideoTranscodeOptions {
    fn default() -> Self {
        Self {
            crf: 26,
            max_videos: MAX_VIDEOS,
        }
    }
}

pub fn is_video_quota_valid(count: usize) -> bool {
    count <= MAX_VIDEOS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_video_options() {
        let opts = VideoTranscodeOptions::default();
        assert_eq!(opts.crf, 26);
        assert_eq!(opts.max_videos, 20);
    }

    #[test]
    fn test_video_quota_validation() {
        assert!(is_video_quota_valid(0));
        assert!(is_video_quota_valid(20));
        assert!(!is_video_quota_valid(21)); // Le rejet de la 21e vidéo exigé par le CDC
    }
}
