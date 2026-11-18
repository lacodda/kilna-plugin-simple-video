//! The timeline a person set by hand for a work, kept by kilna in the work's
//! `meta.simple_video`: what of the track plays, how loud, how it fades in
//! and out, how long the picture stands before the sound and after it.
//!
//! Every value is optional: a timeline that says nothing about the fades
//! keeps the settings' fades. A work with no timeline renders by the
//! settings alone.
//!
//! The shape, as kilna's timeline editor writes it:
//!
//! ```json
//! {
//!   "trim_start": 2.5,       // seconds of the track left out at the start
//!   "trim_end": 181.0,       // the second of the track where it stops
//!   "gain_db": -1.5,         // the track made louder or quieter, dB
//!   "audio_fade_in": 0.5,    // seconds
//!   "audio_fade_out": 4.0,
//!   "video_fade_in": 1.0,    // from black
//!   "video_fade_out": 4.0,   // to black
//!   "lead_in": 1.0,          // seconds of picture, and silence, before the sound
//!   "tail": 2.0              // seconds of picture, and silence, after it
//! }
//! ```

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::settings::Settings;

/// The key the timeline is kept under in a work's `meta`.
pub const META_KEY: &str = "simple_video";

/// A timeline as kept, every value optional.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Timeline {
    pub trim_start: Option<f64>,
    pub trim_end: Option<f64>,
    pub gain_db: Option<f64>,
    pub audio_fade_in: Option<f64>,
    pub audio_fade_out: Option<f64>,
    pub video_fade_in: Option<f64>,
    pub video_fade_out: Option<f64>,
    pub lead_in: Option<f64>,
    pub tail: Option<f64>,
}

impl Timeline {
    /// The timeline a work's `meta` keeps, or none. One that does not read is
    /// none, said by the caller: a render by the settings is better than no
    /// render.
    pub fn of(meta: &Map<String, Value>) -> Result<Option<Self>, String> {
        match meta.get(META_KEY) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => serde_json::from_value(value.clone())
                .map(Some)
                .map_err(|cause| format!("the work's {META_KEY} does not read: {cause}")),
        }
    }
}

/// What a render does, every value settled: the timeline over the settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub trim_start: f64,
    /// Seconds of the track that play.
    pub sound: f64,
    pub gain_db: f64,
    pub audio_fade_in: f64,
    pub audio_fade_out: f64,
    pub video_fade_in: f64,
    pub video_fade_out: f64,
    pub lead_in: f64,
    pub tail: f64,
}

impl Plan {
    /// What a render of a track `length` seconds long does, by `settings`
    /// and the timeline when there is one.
    pub fn new(length: f64, settings: &Settings, timeline: Option<&Timeline>) -> Self {
        let none = Timeline::default();
        let line = timeline.unwrap_or(&none);
        let at_least_zero = |value: Option<f64>, or: f64| value.unwrap_or(or).max(0.0);
        let trim_start = at_least_zero(line.trim_start, 0.0).min(length);
        let end = line
            .trim_end
            .filter(|end| *end > trim_start)
            .unwrap_or(length)
            .min(length);
        let sound = (end - trim_start).max(0.0);
        // A fade is never longer than what it fades.
        let fit = |seconds: f64| seconds.min(sound);
        Self {
            trim_start,
            sound,
            gain_db: line.gain_db.unwrap_or(0.0).clamp(-24.0, 18.0),
            audio_fade_in: fit(at_least_zero(line.audio_fade_in, settings.fade_in)),
            audio_fade_out: fit(at_least_zero(line.audio_fade_out, settings.fade_out)),
            video_fade_in: at_least_zero(line.video_fade_in, settings.fade_in),
            video_fade_out: at_least_zero(line.video_fade_out, settings.fade_out),
            lead_in: at_least_zero(line.lead_in, 0.0),
            tail: at_least_zero(line.tail, 0.0),
        }
    }

    /// How long the video runs: the picture before the sound, the sound, and
    /// the picture after it.
    pub fn total(&self) -> f64 {
        self.lead_in + self.sound + self.tail
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_work_without_a_timeline_renders_by_the_settings() {
        let plan = Plan::new(200.0, &Settings::default(), None);
        assert_eq!(plan.sound, 200.0);
        assert_eq!((plan.audio_fade_out, plan.video_fade_out), (6.0, 6.0));
        assert_eq!(plan.total(), 200.0);
    }

    #[test]
    fn a_timeline_trims_fades_and_holds_the_picture_around_the_sound() {
        let meta = json!({
            "simple_video": {
                "trim_start": 2.5,
                "trim_end": 102.5,
                "gain_db": 40,
                "audio_fade_in": 0.5,
                "lead_in": 1,
                "tail": 2
            }
        });
        let line = Timeline::of(meta.as_object().unwrap()).unwrap().unwrap();
        let plan = Plan::new(200.0, &Settings::default(), Some(&line));
        assert_eq!((plan.trim_start, plan.sound), (2.5, 100.0));
        assert_eq!(
            plan.gain_db, 18.0,
            "the gain stays within the editor's bounds"
        );
        assert_eq!(plan.audio_fade_in, 0.5);
        assert_eq!(plan.audio_fade_out, 6.0, "unsaid, the settings' fade");
        assert_eq!(plan.total(), 103.0);
    }

    #[test]
    fn a_timeline_that_does_not_read_is_said() {
        let meta = json!({ "simple_video": { "trim_start": "soon" } });
        assert!(Timeline::of(meta.as_object().unwrap()).is_err());
        assert_eq!(Timeline::of(&Map::new()).unwrap(), None);
    }
}
