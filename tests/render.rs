//! The plugin against a real ffmpeg: a picture and a tone made by ffmpeg
//! itself are rendered into a video the way kilna asks for one, and the file
//! is measured. Ignored by default - it needs ffmpeg on the PATH; CI runs it
//! with `cargo test -- --ignored`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// A folder of its own for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kilna-plugin-simple-video-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn ffmpeg(args: &[&str]) {
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args)
        .status()
        .expect("ffmpeg runs");
    assert!(status.success(), "ffmpeg {args:?}");
}

fn probe(path: &Path, entries: &str) -> String {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            entries,
            "-of",
            "csv=p=0:s=x",
        ])
        .arg(path)
        .output()
        .expect("ffprobe runs");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Run the plugin as kilna does: `run`, the invocation on stdin.
fn run(invocation: &Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kilna-plugin-simple-video"))
        .arg("run")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the plugin runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(invocation.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("no outcome: {}", String::from_utf8_lossy(&output.stderr)))
}

#[test]
#[ignore = "needs ffmpeg on the PATH"]
fn a_picture_over_a_tone_renders_and_the_tone_is_measured() {
    let dir = scratch("render");
    let cover = dir.join("cover.png");
    let track = dir.join("mix.wav");
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "color=c=steelblue:s=800x800",
        "-frames:v",
        "1",
        cover.to_str().unwrap(),
    ]);
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=4",
        track.to_str().unwrap(),
    ]);
    let subject = json!({
        "title": "A tone: test",
        "kind": "audio",
        "folder": dir.to_str().unwrap(),
        "file": { "state": "found", "path": track.to_str().unwrap() },
        "cover": cover.to_str().unwrap(),
        "meta": { "simple_video": { "lead_in": 1, "tail": 1, "audio_fade_out": 1 } },
        "donors": []
    });

    let outcome = run(&json!({
        "command": "render_headless",
        "target": "work",
        "subject": subject,
        "settings": { "preset": "stream", "fade_out": 1 }
    }));
    assert!(outcome.get("error").is_none(), "{outcome}");
    let made = PathBuf::from(outcome["files"][0].as_str().unwrap());
    assert_eq!(made.file_name().unwrap(), "A tone- test.mp4");
    assert_eq!(
        probe(&made, "stream=width,height").lines().next(),
        Some("1920x1080")
    );
    let seconds: f64 = probe(&made, "format=duration").parse().unwrap();
    assert!(
        (seconds - 6.0).abs() < 0.3,
        "lead-in, tone and tail: {seconds}"
    );

    let measured = run(&json!({
        "command": "analyze_audio",
        "target": "work",
        "subject": subject,
        "settings": {}
    }));
    assert!(measured.get("error").is_none(), "{measured}");
    let loudness = &measured["meta"]["audio_loudness"];
    assert!(
        loudness["integrated_lufs"].as_f64().unwrap() < 0.0,
        "{measured}"
    );
    assert_eq!(loudness["target"], -14.0);
    let _ = std::fs::remove_dir_all(&dir);
}
