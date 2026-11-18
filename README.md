<p align="center">
  <a href="https://github.com/lacodda/kilna-plugin-simple-video/actions"><img src="https://img.shields.io/github/actions/workflow/status/lacodda/kilna-plugin-simple-video/ci.yml?style=flat-square" alt="CI"></a>
  <a href="https://github.com/lacodda/kilna-plugin-simple-video/releases"><img src="https://img.shields.io/github/v/release/lacodda/kilna-plugin-simple-video?style=flat-square" alt="Release"></a>
  <a href="https://github.com/lacodda/kilna-plugin-simple-video/blob/main/LICENSE"><img src="https://img.shields.io/github/license/lacodda/kilna-plugin-simple-video?style=flat-square" alt="License"></a>
</p>

> Renders a simple video for a work of [kilna](https://github.com/lacodda/kilna): its cover - a picture, or a short loop played over and over - over its track, framed, faded, at a stream's 1080p or as a 4K master, into the work's own folder.

*[Русская версия](docs/ru/README.md)*

kilna keeps a work's track and its cover and never opens a video itself. This plugin makes the video: **Render the video** lays `<title>.mp4` in the work's folder, where kilna finds it; **Render by the preset** does the same with no editor opened, meant for many works at once; **Measure the loudness** says how loud the track is and how far from what a platform wants.

## Install

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/lacodda/kilna-plugin-simple-video/main/tools/install.ps1 | iex
```

macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/lacodda/kilna-plugin-simple-video/main/tools/install.sh | sh
```

The installer puts the plugin in kilna's `plugins` folder; its settings stand in kilna's **Settings → Plugins**. It needs **ffmpeg** and **ffprobe** on the `PATH`; without them kilna lists the plugin with the reason instead of its buttons. `KILNA_PLUGIN_SIMPLE_VIDEO_VERSION` pins a release, `KILNA_PLUGIN_SIMPLE_VIDEO_DIR` another folder.

## What a render makes

- **The picture** is the work's cover: a still (`jpg`, `png`, `webp`…) or a loop (`mp4`, `mov`, `webm`…), which plays over and over for as long as the video runs. It is fitted into the frame over black, never cropped.
- **The track** is the work's finished file when that is a track (`wav`, `flac`, `mp3`, `m4a`…), or else the newest track attached to the work on its Files tab (kilna 0.148 and later hands them over), or else the finished track of the first work it is made from - a clip renders over the song's audio.
- **The file** is `<title>.mp4` in the work's folder; a render again writes over it. kilna takes it as the work's finished file where the kind's rule would not find one by itself. A work has a folder once the workspace's media folder is chosen in kilna; until then its video goes to the folder named by `output`, and with none named the render says so.

### Presets

| Preset | Frame | Encoder | Sound |
| --- | --- | --- | --- |
| **Stream, 1080p** | 1920x1080 | `ultrafast`, CRF 18, `-tune stillimage` for a still | AAC 192k |
| **Master, 4K** | 3840x2160 | `medium`, CRF 18 | AAC 320k, the index at the front of the file (`+faststart`) |

Both fade to black and to silence at the end, six seconds unless set otherwise.

### The work's own timeline

A work can keep a timeline of its own in its fields, under `simple_video` - kilna's timeline editor writes it. Every value is optional; what it does not say comes from the settings:

```json
{
  "trim_start": 2.5,
  "trim_end": 181.0,
  "gain_db": -1.5,
  "audio_fade_in": 0.5,
  "audio_fade_out": 4.0,
  "video_fade_in": 1.0,
  "video_fade_out": 4.0,
  "lead_in": 1.0,
  "tail": 2.0
}
```

`trim_start` and `trim_end` are seconds of the track - what plays; `gain_db` makes it louder or quieter, from -24 to +18 dB; the fades are seconds; `lead_in` and `tail` hold the picture, in silence, before the sound and after it. Both renders read it.

## Measuring the loudness

**Measure the loudness** reads the track once with ffmpeg's `loudnorm` and keeps what it found in the work's fields, under `audio_loudness`: `duration`, `integrated_lufs`, `true_peak`, `loudness_range`, the `target` it was measured against and the `gain_to_target` that would bring it there. Nothing of the track is changed.

## Settings

In kilna: **Settings → Plugins**.

| Key | Default | Meaning |
| --- | --- | --- |
| `preset` | `stream` | `stream` (1080p) or `quality` (4K). |
| `codec` | `h264` | `h264` or `h265`. |
| `crf` | `18` | The encoder's CRF, 0 to 51: lower is better, and larger. |
| `fps` | `30` | `30` or `60`. |
| `normalize` | `false` | Bring the track to `loudness`, its peaks under -1 dBTP. |
| `loudness` | `-14` | LUFS: what YouTube and Spotify want. Also what a measure is against. |
| `fade_in` | `0` | Seconds from black and from silence at the start. |
| `fade_out` | `6` | Seconds to black and to silence at the end. |
| `output` | empty | A whole path to a folder for the video of a work kilna names no folder for. |

## Protocol

kilna's plugin protocol 1: `--manifest` prints what the plugin offers - three commands on a work, `render_headless` marked `batch: true` - and `run` reads one invocation from stdin and prints one outcome to stdout; progress goes to stderr as lines of JSON, which kilna draws as a bar.

## Build

```sh
cargo build --release
cargo test
cargo test -- --ignored   # renders with a real ffmpeg
```

## License

MIT
