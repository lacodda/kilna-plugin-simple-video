//! What kilna hands the plugin, and what the plugin hands back.
//!
//! kilna's plugin protocol 1: `--manifest` prints what the plugin offers,
//! `run` reads one invocation from stdin and prints one outcome to stdout. A
//! work travels with its folder, its finished file, its cover, its `meta` and
//! the works it is made from; a command may run for a long time, report
//! progress on stderr and name the files it made. The settings the manifest
//! declares are drawn by kilna and handed over with every run.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::settings::Settings;

/// The protocol this plugin speaks.
pub const PROTOCOL_VERSION: u32 = 1;

/// How long a render may take: a 4K master of a long track on a slow
/// machine. kilna allows no more than an hour.
pub const RENDER_SECONDS: u64 = 3600;

/// How long a measure may take: a track read once.
pub const MEASURE_SECONDS: u64 = 600;

/// What can be set, as kilna draws it. Each default is what a render does
/// when nothing was set, so what the form shows is the truth before anything
/// was.
fn declared(now: &Settings) -> Value {
    json!([
        {
            "key": "preset", "type": "choice", "default": now.preset.key(),
            "label": "Preset", "labels": { "ru": "Пресет" },
            "description": "A stream's 1080p, quick to make, or a 4K master with the sound at 320k.",
            "descriptions": { "ru": "1080p для стримингового канала, быстро, или 4K-мастер со звуком 320k." },
            "options": [
                { "value": "stream", "label": "Stream, 1080p", "labels": { "ru": "Стриминг, 1080p" } },
                { "value": "quality", "label": "Master, 4K", "labels": { "ru": "Мастер, 4K" } }
            ]
        },
        {
            "key": "codec", "type": "choice", "default": now.codec.key(),
            "label": "Codec", "labels": { "ru": "Кодек" },
            "options": [
                { "value": "h264", "label": "H.264" },
                { "value": "h265", "label": "H.265" }
            ]
        },
        {
            "key": "crf", "type": "number", "min": 0, "max": 51, "step": 1, "default": now.crf,
            "label": "Quality", "labels": { "ru": "Качество" },
            "description": "The encoder's CRF: lower is better, and larger.",
            "descriptions": { "ru": "CRF кодека: меньше - лучше и тяжелее." }
        },
        {
            "key": "fps", "type": "choice", "default": now.fps.to_string(),
            "label": "Frames a second", "labels": { "ru": "Кадров в секунду" },
            "options": [
                { "value": "30", "label": "30" },
                { "value": "60", "label": "60" }
            ]
        },
        {
            "key": "normalize", "type": "boolean", "default": now.normalize,
            "label": "Bring the sound to the loudness", "labels": { "ru": "Приводить звук к громкости" },
            "description": "Brings the track to the loudness below, its peaks under -1 dBTP.",
            "descriptions": { "ru": "Приводит трек к громкости ниже, пики под -1 dBTP." }
        },
        {
            "key": "loudness", "type": "number", "min": -40, "max": -5, "step": 0.5,
            "default": now.loudness,
            "label": "Loudness, LUFS", "labels": { "ru": "Громкость, LUFS" },
            "description": "What a platform wants: -14 for YouTube and Spotify.",
            "descriptions": { "ru": "Что хочет площадка: -14 для YouTube и Spotify." }
        },
        {
            "key": "fade_in", "type": "number", "min": 0, "step": 0.5, "default": now.fade_in,
            "label": "Fade in, seconds", "labels": { "ru": "Появление, секунд" },
            "description": "From black and from silence; 0 starts at once.",
            "descriptions": { "ru": "Из черноты и тишины; 0 - сразу." }
        },
        {
            "key": "output", "type": "text", "default": now.output,
            "label": "Where a video goes when the work has no folder",
            "labels": { "ru": "Куда класть видео, если у работы нет папки" },
            "description": "A whole path to a folder. A work has a folder once the workspace's media folder is chosen; until then its video goes here.",
            "descriptions": { "ru": "Полный путь к папке. У работы есть папка, когда выбрана папка медиа пространства; до тех пор видео кладётся сюда." }
        },
        {
            "key": "fade_out", "type": "number", "min": 0, "step": 0.5, "default": now.fade_out,
            "label": "Fade out, seconds", "labels": { "ru": "Затухание, секунд" },
            "description": "To black and to silence at the end; 0 stops at once.",
            "descriptions": { "ru": "В черноту и тишину в конце; 0 - обрывается." }
        }
    ])
}

/// What `--manifest` prints.
pub fn manifest(now: &Settings) -> Value {
    json!({
        "settings": declared(now),
        "protocol_version": PROTOCOL_VERSION,
        "name": "simple-video",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Renders a simple video: the work's cover - a picture or a loop - over its track, framed, faded, at a stream's 1080p or a 4K master.",
        "descriptions": {
            "ru": "Собирает простое видео: обложка работы - картинка или петля - поверх её трека, в кадре, с затуханиями, в 1080p для стриминга или 4K-мастером."
        },
        "commands": [{
            "key": "analyze_audio",
            "label": "Measure the loudness",
            "labels": { "ru": "Измерить громкость" },
            "description": "Measures the track's loudness and peaks, and how far it is from the loudness set. Nothing is changed.",
            "descriptions": { "ru": "Измеряет громкость и пики трека и насколько он далёк от заданной громкости. Ничего не меняет." },
            "target": "work",
            "timeout": MEASURE_SECONDS
        }, {
            "key": "render",
            "label": "Render the video",
            "labels": { "ru": "Собрать видео" },
            "description": "Renders the cover over the track into <title>.mp4 in the work's folder, by the work's timeline when it has one.",
            "descriptions": { "ru": "Собирает обложку поверх трека в <название>.mp4 в папке работы, по таймлайну работы, если он есть." },
            "target": "work",
            "timeout": RENDER_SECONDS
        }, {
            "key": "render_headless",
            "label": "Render by the preset",
            "labels": { "ru": "Собрать по пресету" },
            "description": "The same render with no editor opened, for many works at once: the preset, or each work's own timeline.",
            "descriptions": { "ru": "Та же сборка без редактора, на много работ сразу: по пресету или по таймлайну каждой работы." },
            "target": "work",
            "batch": true,
            "timeout": RENDER_SECONDS
        }]
    })
}

/// One invocation, as kilna writes it to stdin.
#[derive(Debug, Deserialize)]
pub struct Invocation {
    pub command: String,
    pub subject: Subject,
    /// What was set for this plugin in kilna, or the manifest's defaults.
    #[serde(default)]
    pub settings: Map<String, Value>,
}

/// The work a command runs on - only what a renderer reads of it.
#[derive(Debug, Default, Deserialize)]
pub struct Subject {
    #[serde(default)]
    pub title: String,
    /// The work's folder on disk, whole, when its kind names one.
    #[serde(default)]
    pub folder: Option<String>,
    /// Its finished file, as kilna describes it.
    #[serde(default)]
    pub file: Option<FinishedFile>,
    /// The picture it goes out under, whole, when it has one.
    #[serde(default)]
    pub cover: Option<String>,
    /// Its fields: the timeline kept under `simple_video`.
    #[serde(default)]
    pub meta: Map<String, Value>,
    /// The works it is made from, each with its finished file.
    #[serde(default)]
    pub donors: Vec<Donor>,
    /// The files attached to it, oldest first (kilna v0.148): a track kept
    /// on the work's Files tab rather than in its folder.
    #[serde(default)]
    pub files: Vec<Attached>,
}

/// A file attached to a work.
#[derive(Debug, Clone, Deserialize)]
pub struct Attached {
    pub path: String,
    /// What the world outside calls it: the name it arrived under.
    #[serde(default)]
    pub name: Option<String>,
}

impl Subject {
    /// The work's finished file, whole, when it has one.
    pub fn file_path(&self) -> Option<&str> {
        self.file.as_ref().and_then(FinishedFile::path)
    }
}

/// A work this one is made from.
#[derive(Debug, Clone, Deserialize)]
pub struct Donor {
    #[serde(default)]
    pub file: Option<FinishedFile>,
}

impl Donor {
    pub fn path(&self) -> Option<&str> {
        self.file.as_ref().and_then(FinishedFile::path)
    }
}

/// A finished file as kilna hands it over: an object with its whole `path`
/// when it has one - or a bare path, which a person writing an invocation by
/// hand may give instead.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FinishedFile {
    Path(String),
    Described {
        #[serde(default)]
        path: Option<String>,
    },
}

impl FinishedFile {
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Path(path) => Some(path),
            Self::Described { path } => path.as_deref(),
        }
    }
}

/// What the plugin prints when it is done.
#[derive(Debug, Default, Serialize)]
pub struct Outcome {
    /// Fields merged into the work's `meta`.
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub meta: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

/// Say how far along the run is: one line of JSON on stderr, which kilna
/// draws as a bar under the command.
pub fn progress(share: f64, message: &str) {
    let line = json!({
        "progress": share.clamp(0.0, 1.0),
        "message": message,
    });
    eprintln!("{line}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_offers_a_measure_a_render_and_a_render_for_many_works() {
        let manifest = manifest(&Settings::default());
        assert_eq!(manifest["protocol_version"], 1);
        let commands = manifest["commands"].as_array().unwrap();
        let keys: Vec<&str> = commands
            .iter()
            .map(|command| command["key"].as_str().unwrap())
            .collect();
        assert_eq!(keys, ["analyze_audio", "render", "render_headless"]);
        for command in commands {
            assert_eq!(command["target"], "work");
            let allowance = command["timeout"].as_u64().unwrap();
            assert!(allowance > 120 && allowance <= 3600);
        }
        assert_eq!(commands[2]["batch"], true);
        let settings = manifest["settings"].as_array().unwrap();
        assert!(
            settings
                .iter()
                .any(|one| one["key"] == "preset" && one["default"] == "stream")
        );
        assert!(
            settings
                .iter()
                .any(|one| one["key"] == "loudness" && one["default"] == -14.0)
        );
    }

    #[test]
    fn an_invocation_as_kilna_writes_it_reads() {
        let written = json!({
            "command": "render",
            "target": "work",
            "subject": {
                "id": "w-1",
                "title": "Song",
                "kind": "audio",
                "folder": "C:/space/media/w-1",
                "file": { "state": "found", "rule": "audio", "relative": "mix.wav", "path": "C:/space/media/w-1/mix.wav" },
                "cover": "C:/space/media/w-1/cover.png",
                "meta": { "simple_video": { "tail": 2 } },
                "donors": [{ "id": "w-0", "title": "Song", "kind": "song", "folder": null, "file": null }],
                "files": [{ "kind": "attachment", "path": "C:/space/media/a1.wav", "name": "mix.wav" }]
            },
            "settings": { "preset": "quality" }
        });
        let invocation: Invocation = serde_json::from_value(written).unwrap();
        assert_eq!(
            invocation.subject.file_path(),
            Some("C:/space/media/w-1/mix.wav")
        );
        assert_eq!(invocation.subject.donors[0].path(), None);
        assert!(invocation.subject.meta.contains_key("simple_video"));
        assert_eq!(invocation.subject.files[0].name.as_deref(), Some("mix.wav"));
    }
}
