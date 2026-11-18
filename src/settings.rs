//! What can be set for a render, as kilna hands it over with every run.
//!
//! kilna draws the settings the manifest declares, keeps what was set and
//! gives the values with each invocation; a value missing or of the wrong
//! shape falls back to the default rather than failing the render.

use serde_json::{Map, Value};

/// Where the video goes out: a stream's 1080p, quick to make, or a 4K master.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// 1920x1080, ultrafast, AAC at 192k, a fade-out at the end.
    Stream,
    /// 3840x2160, a slower encode, AAC at 320k, the index at the front of the
    /// file so it plays before it has all arrived.
    Quality,
}

impl Preset {
    pub fn key(self) -> &'static str {
        match self {
            Self::Stream => "stream",
            Self::Quality => "quality",
        }
    }

    /// The frame, width by height.
    pub fn frame(self) -> (u32, u32) {
        match self {
            Self::Stream => (1920, 1080),
            Self::Quality => (3840, 2160),
        }
    }

    /// How hard the encoder works: a stream's picture stands still, so the
    /// fastest is enough; a master can wait.
    pub fn speed(self) -> &'static str {
        match self {
            Self::Stream => "ultrafast",
            Self::Quality => "medium",
        }
    }

    /// The bitrate of the sound.
    pub fn audio_bitrate(self) -> &'static str {
        match self {
            Self::Stream => "192k",
            Self::Quality => "320k",
        }
    }
}

/// The video codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    H264,
    H265,
}

impl Codec {
    pub fn key(self) -> &'static str {
        match self {
            Self::H264 => "h264",
            Self::H265 => "h265",
        }
    }

    /// The encoder ffmpeg is told to use.
    pub fn encoder(self) -> &'static str {
        match self {
            Self::H264 => "libx264",
            Self::H265 => "libx265",
        }
    }
}

/// Everything a render reads of the settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub preset: Preset,
    pub codec: Codec,
    /// The encoder's CRF: lower is better, and larger.
    pub crf: u32,
    /// Frames a second of the video made.
    pub fps: u32,
    /// The loudness the sound is brought to when it is normalised, LUFS.
    pub loudness: f64,
    /// Whether the sound is brought to `loudness`.
    pub normalize: bool,
    /// Seconds of fade from black and from silence at the start.
    pub fade_in: f64,
    /// Seconds of fade to black and to silence at the end.
    pub fade_out: f64,
    /// A folder, whole, for the video of a work kilna names no folder for:
    /// a workspace with no media folder chosen. Empty: such a work is not
    /// rendered, and says why.
    pub output: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            preset: Preset::Stream,
            codec: Codec::H264,
            crf: 18,
            fps: 30,
            loudness: -14.0,
            normalize: false,
            fade_in: 0.0,
            fade_out: 6.0,
            output: String::new(),
        }
    }
}

impl Settings {
    /// The settings kilna handed over, each missing or unreadable one at its
    /// default.
    pub fn from(given: &Map<String, Value>) -> Self {
        let base = Self::default();
        let text = |key: &str| given.get(key).and_then(Value::as_str);
        let number = |key: &str| {
            given.get(key).and_then(|value| {
                value
                    .as_f64()
                    .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
            })
        };
        Self {
            preset: match text("preset") {
                Some("quality") => Preset::Quality,
                _ => Preset::Stream,
            },
            codec: match text("codec") {
                Some("h265") => Codec::H265,
                _ => Codec::H264,
            },
            crf: number("crf")
                .filter(|crf| (0.0..=51.0).contains(crf))
                .map_or(base.crf, |crf| crf.round() as u32),
            fps: match number("fps").map(|fps| fps.round() as u32) {
                Some(60) => 60,
                _ => 30,
            },
            loudness: number("loudness")
                .filter(|lufs| (-40.0..=-5.0).contains(lufs))
                .unwrap_or(base.loudness),
            normalize: given
                .get("normalize")
                .and_then(Value::as_bool)
                .unwrap_or(base.normalize),
            fade_in: number("fade_in")
                .filter(|seconds| *seconds >= 0.0)
                .unwrap_or(base.fade_in),
            fade_out: number("fade_out")
                .filter(|seconds| *seconds >= 0.0)
                .unwrap_or(base.fade_out),
            output: text("output").map(str::trim).unwrap_or_default().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn what_kilna_hands_over_is_read_and_a_wrong_value_falls_back() {
        let given = json!({
            "preset": "quality",
            "codec": "h265",
            "crf": 20,
            "fps": "60",
            "loudness": -16,
            "normalize": true,
            "fade_in": 1.5,
            "fade_out": -3,
        });
        let settings = Settings::from(given.as_object().unwrap());
        assert_eq!(settings.preset, Preset::Quality);
        assert_eq!(settings.codec, Codec::H265);
        assert_eq!((settings.crf, settings.fps), (20, 60));
        assert_eq!(settings.loudness, -16.0);
        assert!(settings.normalize);
        assert_eq!(settings.fade_in, 1.5);
        assert_eq!(settings.fade_out, 6.0, "a negative fade is no fade asked");
        assert_eq!(Settings::from(&Map::new()), Settings::default());
    }
}
