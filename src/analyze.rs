//! How loud a track is: its integrated loudness, its true peak, its range,
//! and how far it is from the loudness a platform wants - measured by
//! ffmpeg's `loudnorm` in its first, measuring pass.

use std::path::Path;

use serde::Serialize;

use crate::ffmpeg::{self, Tools};

/// What was measured of a track.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Loudness {
    /// Seconds.
    pub duration: f64,
    /// Integrated loudness, LUFS.
    pub integrated_lufs: f64,
    /// True peak, dBTP.
    pub true_peak: f64,
    /// Loudness range, LU.
    pub loudness_range: f64,
    /// The loudness it was measured against, LUFS.
    pub target: f64,
    /// How many dB the track is from the target: what a gain would add.
    pub gain_to_target: f64,
}

/// Measure `track` against `target` LUFS.
pub fn measure(tools: &Tools, track: &Path, target: f64) -> Result<Loudness, String> {
    let duration = ffmpeg::length(tools, track)?;
    let said = ffmpeg::said(
        tools,
        &[
            "-i".to_owned(),
            track.to_string_lossy().into_owned(),
            "-af".to_owned(),
            format!("loudnorm=I={target}:TP=-1:LRA=11:print_format=json"),
            "-f".to_owned(),
            "null".to_owned(),
            "-".to_owned(),
        ],
    )?;
    read(&said, duration, target)
}

/// The findings `loudnorm` prints: the last JSON object of what ffmpeg said.
pub fn read(said: &str, duration: f64, target: f64) -> Result<Loudness, String> {
    let start = said
        .rfind('{')
        .ok_or("ffmpeg measured nothing: no loudness came back")?;
    let end = said[start..]
        .find('}')
        .map(|at| start + at + 1)
        .ok_or("ffmpeg's measure does not read")?;
    let found: serde_json::Value = serde_json::from_str(&said[start..end])
        .map_err(|cause| format!("ffmpeg's measure does not read: {cause}"))?;
    // loudnorm says its numbers as strings, and "-inf" for silence.
    let number = |key: &str| -> Result<f64, String> {
        let text = found[key]
            .as_str()
            .ok_or_else(|| format!("ffmpeg's measure has no {key}"))?;
        text.trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| "the track is silent: there is no loudness to measure".to_owned())
    };
    let integrated = number("input_i")?;
    Ok(Loudness {
        duration,
        integrated_lufs: integrated,
        true_peak: number("input_tp")?,
        loudness_range: number("input_lra").unwrap_or(0.0),
        target,
        gain_to_target: ((target - integrated) * 10.0).round() / 10.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAID: &str = r#"[Parsed_loudnorm_0 @ 0x55]
{
	"input_i" : "-18.24",
	"input_tp" : "-0.31",
	"input_lra" : "6.20",
	"input_thresh" : "-28.50",
	"output_i" : "-14.02",
	"output_tp" : "-1.00",
	"output_lra" : "5.10",
	"output_thresh" : "-24.30",
	"normalization_type" : "dynamic",
	"target_offset" : "0.02"
}
"#;

    #[test]
    fn the_measure_ffmpeg_prints_is_read_and_the_gain_to_the_target_said() {
        let loudness = read(SAID, 201.5, -14.0).unwrap();
        assert_eq!(loudness.integrated_lufs, -18.24);
        assert_eq!(loudness.true_peak, -0.31);
        assert_eq!(loudness.loudness_range, 6.2);
        assert_eq!(loudness.gain_to_target, 4.2);
    }

    #[test]
    fn a_silent_track_is_said_rather_than_measured_as_infinity() {
        let silent = SAID.replace("\"-18.24\"", "\"-inf\"");
        assert!(read(&silent, 3.0, -14.0).unwrap_err().contains("silent"));
    }
}
