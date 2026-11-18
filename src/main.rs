//! kilna-plugin-simple-video: renders a simple video for a work of kilna.
//!
//! A kilna plugin: an executable named `kilna-plugin-*`, run once per call.
//! `--manifest` says what it offers; `run` reads the invocation from stdin
//! and either measures the work's track or renders its cover over its track
//! with ffmpeg into `<title>.mp4` in the work's folder; it reports progress
//! on stderr and answers on stdout. kilna itself never opens a video.

mod analyze;
mod ffmpeg;
mod protocol;
mod render;
mod settings;
mod timeline;

use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use protocol::{Invocation, Outcome, Subject};
use settings::Settings;

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("--manifest") => describe(),
        Some("run") => {
            let outcome = run();
            match serde_json::to_string(&outcome) {
                Ok(line) => {
                    println!("{line}");
                    ExitCode::SUCCESS
                }
                Err(cause) => {
                    eprintln!("could not write the outcome: {cause}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("--version") => {
            println!("kilna-plugin-simple-video {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "kilna-plugin-simple-video {}: a kilna plugin. kilna runs it as `--manifest` and `run`.",
                env!("CARGO_PKG_VERSION")
            );
            ExitCode::from(2)
        }
    }
}

/// Print the manifest - unless ffmpeg is missing, said on stderr so kilna
/// lists the plugin with the reason.
fn describe() -> ExitCode {
    if let Err(reason) = ffmpeg::find() {
        eprintln!("{reason}");
        return ExitCode::FAILURE;
    }
    println!("{}", protocol::manifest(&Settings::default()));
    ExitCode::SUCCESS
}

/// One call: read, do, answer. A failure is answered as `error`, in words a
/// person can act on, rather than as a crash.
fn run() -> Outcome {
    let mut input = String::new();
    if let Err(cause) = std::io::stdin().read_to_string(&mut input) {
        return failed(format!("could not read the invocation: {cause}"));
    }
    let invocation: Invocation = match serde_json::from_str(&input) {
        Ok(invocation) => invocation,
        Err(cause) => return failed(format!("the invocation does not read: {cause}")),
    };
    let tools = match ffmpeg::find() {
        Ok(tools) => tools,
        Err(reason) => return failed(reason),
    };
    let settings = Settings::from(&invocation.settings);
    match invocation.command.as_str() {
        "analyze_audio" => measure(&invocation.subject, &settings, &tools),
        "render" | "render_headless" => make(&invocation.subject, &settings, &tools),
        other => failed(format!("there is no command \"{other}\" here")),
    }
}

/// Measure the work's track and keep what was found in its fields.
fn measure(subject: &Subject, settings: &Settings, tools: &ffmpeg::Tools) -> Outcome {
    let track = match render::sources(subject) {
        Ok((_, track)) => track,
        // A track with no cover yet is still a track to measure.
        Err(_) => match subject.file_path() {
            Some(path) => path.into(),
            None => return failed("the work has no finished track to measure".into()),
        },
    };
    protocol::progress(0.0, "Measuring the loudness");
    match analyze::measure(tools, Path::new(&track), settings.loudness) {
        Ok(found) => {
            protocol::progress(1.0, "Done");
            let mut meta = serde_json::Map::new();
            meta.insert(
                "audio_loudness".into(),
                serde_json::to_value(&found).unwrap_or_default(),
            );
            Outcome {
                message: Some(format!(
                    "{:.1} LUFS, peak {:.1} dBTP: {:+.1} dB to {} LUFS",
                    found.integrated_lufs, found.true_peak, found.gain_to_target, found.target
                )),
                meta,
                ..Outcome::default()
            }
        }
        Err(reason) => failed(reason),
    }
}

/// Render the work's video into its folder.
fn make(subject: &Subject, settings: &Settings, tools: &ffmpeg::Tools) -> Outcome {
    protocol::progress(0.0, "Rendering");
    let made = render::render(subject, settings, tools, &mut |share| {
        protocol::progress(share, "Rendering");
    });
    match made {
        Ok(file) => {
            protocol::progress(1.0, "Done");
            let name = file
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let length = ffmpeg::length(tools, &file).ok();
            Outcome {
                message: Some(match length {
                    Some(seconds) => format!("Rendered {name}: {}", clock(seconds)),
                    None => format!("Rendered {name}"),
                }),
                files: vec![file.to_string_lossy().into_owned()],
                ..Outcome::default()
            }
        }
        Err(reason) => failed(reason),
    }
}

fn failed(reason: String) -> Outcome {
    Outcome {
        error: Some(reason),
        ..Outcome::default()
    }
}

/// Seconds as a clock: `3:21`.
fn clock(seconds: f64) -> String {
    let whole = seconds.max(0.0).round() as u64;
    format!("{}:{:02}", whole / 60, whole % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_read_as_a_clock() {
        assert_eq!(clock(201.4), "3:21");
        assert_eq!(clock(59.6), "1:00");
    }
}
