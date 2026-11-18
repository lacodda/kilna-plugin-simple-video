//! A simple video made with ffmpeg: a picture - or a short loop, played over
//! and over - for as long as the track plays, framed in the preset's frame
//! over black, with its fades, and the file put in the work's folder.

use std::path::{Path, PathBuf};

use crate::ffmpeg::{self, Tools};
use crate::protocol::Subject;
use crate::settings::Settings;
use crate::timeline::{Plan, Timeline};

/// Files a picture can be.
const STILLS: [&str; 6] = ["jpg", "jpeg", "png", "webp", "bmp", "gif"];
/// Files a loop can be.
const LOOPS: [&str; 4] = ["mp4", "mov", "webm", "mkv"];
/// Files a track can be.
const TRACKS: [&str; 8] = ["wav", "flac", "mp3", "m4a", "aac", "ogg", "opus", "aiff"];

fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// What the picture is.
#[derive(Debug, Clone, PartialEq)]
pub enum Picture {
    Still(PathBuf),
    Loop(PathBuf),
}

/// The picture and the track of a work: its cover - a picture or a loop -
/// and its finished file when that is a track, or else the first work it is
/// made from whose finished file is one.
pub fn sources(subject: &Subject) -> Result<(Picture, PathBuf), String> {
    let cover = subject
        .cover
        .as_deref()
        .ok_or("the work has no cover to show: set one on its Cover tab")?;
    let picture = match extension(cover).as_str() {
        ext if STILLS.contains(&ext) => Picture::Still(PathBuf::from(cover)),
        ext if LOOPS.contains(&ext) => Picture::Loop(PathBuf::from(cover)),
        _ => return Err(format!("{cover} is neither a picture nor a loop")),
    };
    let is_track = |path: &&str| TRACKS.contains(&extension(path).as_str());
    let track = subject
        .file_path()
        .filter(is_track)
        .or_else(|| {
            subject
                .donors
                .iter()
                .filter_map(|donor| donor.path())
                .find(is_track)
        })
        .ok_or("neither the work nor what it is made from has a finished track")?;
    Ok((picture, PathBuf::from(track)))
}

/// The file a work's video goes to: its title, in its folder.
pub fn output(subject: &Subject) -> Result<PathBuf, String> {
    let folder = subject
        .folder
        .as_deref()
        .ok_or("the work has no folder to put the video in")?;
    Ok(Path::new(folder).join(format!("{}.mp4", file_name(&subject.title))))
}

/// A title as a file's name: what no system takes in a name made a dash.
pub fn file_name(title: &str) -> String {
    let named: String = title
        .trim()
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    let named = named.trim_end_matches(['.', ' ']).to_owned();
    if named.is_empty() {
        "video".to_owned()
    } else {
        named
    }
}

/// Seconds as ffmpeg reads them.
fn secs(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    format!("{rounded}")
}

/// The arguments of one render, everything settled: what is read, how the
/// picture is framed and faded, how the sound is cut, made louder, faded and
/// held, how it is encoded and where it goes.
pub fn args(
    picture: &Picture,
    track: &Path,
    out: &Path,
    plan: &Plan,
    settings: &Settings,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut push = |items: &[&str]| args.extend(items.iter().map(|item| (*item).to_owned()));
    match picture {
        // One frame a second in: the picture does not move, and the frame
        // rate the video is made at is set after it is framed.
        Picture::Still(path) => {
            push(&["-loop", "1", "-framerate", "1", "-i"]);
            args.push(path.to_string_lossy().into_owned());
        }
        Picture::Loop(path) => {
            push(&["-stream_loop", "-1", "-i"]);
            args.push(path.to_string_lossy().into_owned());
        }
    }
    let mut push = |items: &[&str]| args.extend(items.iter().map(|item| (*item).to_owned()));
    if plan.trim_start > 0.0 {
        push(&["-ss"]);
        args.push(secs(plan.trim_start));
    }
    args.extend(["-t".to_owned(), secs(plan.sound), "-i".to_owned()]);
    args.push(track.to_string_lossy().into_owned());

    let total = plan.total();
    let (width, height) = settings.preset.frame();
    let mut video = vec![
        format!("scale={width}:{height}:force_original_aspect_ratio=decrease"),
        format!("pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=black"),
        "setsar=1".to_owned(),
        format!("fps={}", settings.fps),
    ];
    if plan.video_fade_in > 0.0 {
        video.push(format!("fade=t=in:st=0:d={}", secs(plan.video_fade_in)));
    }
    if plan.video_fade_out > 0.0 {
        let start = (total - plan.video_fade_out).max(0.0);
        video.push(format!(
            "fade=t=out:st={}:d={}",
            secs(start),
            secs(plan.video_fade_out)
        ));
    }
    video.push("format=yuv420p".to_owned());

    let mut audio = Vec::new();
    if plan.gain_db != 0.0 {
        audio.push(format!("volume={}dB", secs(plan.gain_db)));
    }
    if settings.normalize {
        audio.push(format!(
            "loudnorm=I={}:TP=-1:LRA=11",
            secs(settings.loudness)
        ));
    }
    if plan.audio_fade_in > 0.0 {
        audio.push(format!("afade=t=in:st=0:d={}", secs(plan.audio_fade_in)));
    }
    if plan.audio_fade_out > 0.0 {
        let start = (plan.sound - plan.audio_fade_out).max(0.0);
        audio.push(format!(
            "afade=t=out:st={}:d={}",
            secs(start),
            secs(plan.audio_fade_out)
        ));
    }
    if plan.lead_in > 0.0 {
        let millis = (plan.lead_in * 1000.0).round() as u64;
        audio.push(format!("adelay=delays={millis}:all=1"));
    }
    // The sound is held for the whole video, the tail silent.
    audio.push(format!("apad=whole_dur={}", secs(total)));

    let filter = format!("[0:v]{}[v];[1:a]{}[a]", video.join(","), audio.join(","));
    args.extend([
        "-filter_complex".to_owned(),
        filter,
        "-map".to_owned(),
        "[v]".to_owned(),
        "-map".to_owned(),
        "[a]".to_owned(),
        "-t".to_owned(),
        secs(total),
        "-c:v".to_owned(),
        settings.codec.encoder().to_owned(),
        "-preset".to_owned(),
        settings.preset.speed().to_owned(),
        "-crf".to_owned(),
        settings.crf.to_string(),
    ]);
    if matches!(picture, Picture::Still(_)) && settings.codec == crate::settings::Codec::H264 {
        args.extend(["-tune".to_owned(), "stillimage".to_owned()]);
    }
    args.extend([
        "-c:a".to_owned(),
        "aac".to_owned(),
        "-b:a".to_owned(),
        settings.preset.audio_bitrate().to_owned(),
    ]);
    if settings.preset == crate::settings::Preset::Quality {
        args.extend(["-movflags".to_owned(), "+faststart".to_owned()]);
    }
    args.push(out.to_string_lossy().into_owned());
    args
}

/// Render the work's video into its folder, telling how far along it is.
/// The file made comes back.
pub fn render(
    subject: &Subject,
    settings: &Settings,
    tools: &Tools,
    on_share: &mut dyn FnMut(f64),
) -> Result<PathBuf, String> {
    let (picture, track) = sources(subject)?;
    let out = output(subject)?;
    let timeline = Timeline::of(&subject.meta)?;
    let length = ffmpeg::length(tools, &track)?;
    let plan = Plan::new(length, settings, timeline.as_ref());
    if plan.sound <= 0.0 {
        return Err("the timeline leaves nothing of the track to play".into());
    }
    if let Some(folder) = out.parent() {
        std::fs::create_dir_all(folder)
            .map_err(|cause| format!("could not make {}: {cause}", folder.display()))?;
    }
    let total = plan.total();
    let args = args(&picture, &track, &out, &plan, settings);
    ffmpeg::run(tools, &args, &mut |seconds| on_share(seconds / total))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Codec, Preset};

    fn joined(args: &[String]) -> String {
        args.join(" ")
    }

    #[test]
    fn a_still_over_a_track_renders_at_the_streams_1080p_with_its_fade_out() {
        let plan = Plan::new(200.0, &Settings::default(), None);
        let made = args(
            &Picture::Still("cover.png".into()),
            Path::new("mix.wav"),
            Path::new("out/Song.mp4"),
            &plan,
            &Settings::default(),
        );
        let said = joined(&made);
        assert!(
            said.starts_with("-loop 1 -framerate 1 -i cover.png -t 200 -i mix.wav"),
            "{said}"
        );
        assert!(said.contains("scale=1920:1080:force_original_aspect_ratio=decrease"));
        assert!(said.contains("fps=30"));
        assert!(said.contains("fade=t=out:st=194:d=6"), "{said}");
        assert!(said.contains("afade=t=out:st=194:d=6"), "{said}");
        assert!(said.contains("-c:v libx264 -preset ultrafast -crf 18 -tune stillimage"));
        assert!(said.contains("-c:a aac -b:a 192k"));
        assert!(!said.contains("loudnorm") && !said.contains("faststart"));
        assert!(said.ends_with("out/Song.mp4") || said.ends_with("out\\Song.mp4"));
    }

    #[test]
    fn the_quality_preset_makes_a_4k_master_brought_to_the_loudness() {
        let settings = Settings {
            preset: Preset::Quality,
            codec: Codec::H265,
            normalize: true,
            fade_in: 1.0,
            ..Settings::default()
        };
        let plan = Plan::new(60.0, &settings, None);
        let said = joined(&args(
            &Picture::Loop("loop.mp4".into()),
            Path::new("mix.flac"),
            Path::new("Song.mp4"),
            &plan,
            &settings,
        ));
        assert!(said.starts_with("-stream_loop -1 -i loop.mp4"), "{said}");
        assert!(said.contains("scale=3840:2160"));
        assert!(said.contains("loudnorm=I=-14:TP=-1:LRA=11"));
        assert!(said.contains("fade=t=in:st=0:d=1"));
        assert!(said.contains("-c:v libx265 -preset medium"));
        assert!(!said.contains("stillimage"), "a loop moves");
        assert!(said.contains("-b:a 320k") && said.contains("-movflags +faststart"));
    }

    #[test]
    fn a_timeline_trims_the_track_and_holds_the_picture_around_it() {
        let line = Timeline {
            trim_start: Some(2.5),
            trim_end: Some(62.5),
            gain_db: Some(-3.0),
            lead_in: Some(1.0),
            tail: Some(2.0),
            ..Timeline::default()
        };
        let plan = Plan::new(200.0, &Settings::default(), Some(&line));
        let said = joined(&args(
            &Picture::Still("cover.jpg".into()),
            Path::new("mix.wav"),
            Path::new("Song.mp4"),
            &plan,
            &Settings::default(),
        ));
        assert!(said.contains("-ss 2.5 -t 60 -i mix.wav"), "{said}");
        assert!(said.contains("volume=-3dB"));
        assert!(said.contains("adelay=delays=1000:all=1"));
        assert!(said.contains("apad=whole_dur=63"));
        assert!(said.contains("-t 63 "), "{said}");
        assert!(
            said.contains("fade=t=out:st=57:d=6"),
            "the picture fades at the very end"
        );
        assert!(
            said.contains("afade=t=out:st=54:d=6"),
            "the sound fades at its own end"
        );
    }

    #[test]
    fn a_title_becomes_a_name_every_system_takes() {
        assert_eq!(file_name("Ты - моя осень"), "Ты - моя осень");
        assert_eq!(file_name("A/B: c?"), "A-B- c-");
        assert_eq!(file_name("  ...  "), "video");
    }
}
