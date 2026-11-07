//! FFmpeg-based export backend (Path A of M7).
//!
//! Rendered frames from the shared [`crate::pipeline::Pipeline`] are piped as rawvideo to an
//! ffmpeg child process, which encodes and muxes the output. Audio is mixed by the existing
//! [`crate::audio_out::AudioOut`] and piped as raw PCM to the same ffmpeg instance.
//!
//! This keeps pixel-in-pixel parity with the preview: the same compositor renders every frame,
//! ffmpeg only encodes what it receives.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;

use filmcraft_project::{ItemId, Project};
use filmcraft_render::SourceProvider;
use filmcraft_time::{FrameRate, Tick};

use image::ImageEncoder as _;

use crate::audio_out::AudioOut;
use crate::format::ExportSettings;
use crate::pipeline::Pipeline;
use crate::{ExportError, Progress, Report, Result};

/// Which video codec ffmpeg should use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FfmpegCodec {
    #[default]
    H264,
    Hevc,
    Vp9,
}

/// Whether ffmpeg is available on this machine and which video codecs it can encode.
#[derive(Clone, Debug, Default)]
pub struct FfmpegProbe {
    pub path: Option<String>,
    pub h264: bool,
    pub hevc: bool,
    pub vp9: bool,
    pub aac: bool,
    pub nvenc: bool,
    pub videotoolbox: bool,
    pub qsv: bool,
}

/// Probe the machine for ffmpeg and its encoders. Runs `ffmpeg -version` and `ffmpeg -encoders`.
pub fn probe() -> FfmpegProbe {
    let mut out = FfmpegProbe::default();
    if let Some(p) = which_ffmpeg() {
        out.path = Some(p.clone());
        let encoders = encoders_list(&p);
        out.h264 = encoders.contains("libx264") || encoders.contains("h264_nvenc") || encoders.contains("h264_videotoolbox") || encoders.contains("h264_qsv");
        out.hevc = encoders.contains("libx265") || encoders.contains("hevc_nvenc") || encoders.contains("hevc_videotoolbox") || encoders.contains("hevc_qsv");
        out.vp9 = encoders.contains("libvpx-vp9");
        out.aac = encoders.contains("aac");
        out.nvenc = encoders.contains("h264_nvenc") || encoders.contains("hevc_nvenc");
        out.videotoolbox = encoders.contains("h264_videotoolbox") || encoders.contains("hevc_videotoolbox");
        out.qsv = encoders.contains("h264_qsv") || encoders.contains("hevc_qsv");
    }
    out
}

/// Find the ffmpeg binary on PATH or in common locations.
fn which_ffmpeg() -> Option<String> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split([':', ';']).filter(|s| !s.is_empty()) {
            for name in ["ffmpeg", "ffmpeg.exe"] {
                let p = std::path::Path::new(dir).join(name);
                if p.is_file() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
        }
    }
    for p in [
        "/usr/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/opt/homebrew/bin/ffmpeg",
        "/opt/local/bin/ffmpeg",
        "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
    ] {
        if std::path::Path::new(p).is_file() {
            return Some(p.to_string());
        }
    }
    None
}

/// List encoder names from `ffmpeg -encoders`.
fn encoders_list(ffmpeg: &str) -> String {
    Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

/// A source provider that returns silence/black (used for still export where no media sources
/// are needed — the pipeline renders the sequence from its own media, so we need a provider
/// that the pipeline can query for decode; the pipeline itself handles source lookup).
struct EmptyProvider;

impl SourceProvider for EmptyProvider {
    fn source(&self, _item: ItemId) -> Option<filmcraft_media::SharedSource> {
        None
    }
}

/// Run an ffmpeg export: render frames with the shared pipeline, pipe them and the mixed audio
/// to ffmpeg, which encodes and muxes.
///
/// `progress` reports frame-level progress and supports cancellation.
pub fn export_ffmpeg(
    project: &Arc<Project>,
    seq: ItemId,
    settings: &ExportSettings,
    sources: &dyn SourceProvider,
    progress: &Progress,
) -> Result<Report> {
    let probed = probe();
    let ffmpeg_path = settings
        .ffmpeg_path
        .as_deref()
        .or_else(|| probed.path.as_deref())
        .ok_or_else(|| ExportError::Unsupported("ffmpeg not found: set the path in Settings > Export".into()))?;
    let ffmpeg = ffmpeg_path;

    let pipe = Pipeline::new(project.clone(), seq, settings, false)?;
    let range = crate::export_range(project, seq, settings)?;
    let rate = pipe.rate;
    let (f0, f1) = crate::frame_span(rate, range);
    let total_frames = (f1 - f0) as u64;
    if total_frames == 0 {
        return Err(ExportError::Unsupported("export range has no frames".into()));
    }

    let (w, h) = (pipe.w, pipe.h);

    let mut cmd = build_command(ffmpeg, settings, w, h, rate)?;

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ExportError::Io(format!("ffmpeg spawn: {e}")))?;

    let stdin = child.stdin.take().ok_or_else(|| ExportError::Io("ffmpeg stdin".into()))?;
    let stdin = Arc::new(Mutex::new(stdin));

    let stderr = child.stderr.take().ok_or_else(|| ExportError::Io("ffmpeg stderr".into()))?;
    let err_out = Arc::new(Mutex::new(String::new()));
    let err_out_clone = err_out.clone();
    thread::spawn(move || {
        use std::io::BufRead;
        let mut buf = String::new();
        let mut reader = std::io::BufReader::new(stderr);
        loop {
            buf.clear();
            match reader.read_line(&mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    err_out_clone.lock().unwrap().push_str(&buf);
                }
                Err(_) => break,
            }
        }
    });

    let cancelled = || progress.cancel.load(Ordering::Relaxed);
    let batch = rayon::current_num_threads().clamp(2, 16) as i64;
    let mut f = f0;
    let mut written: u64 = 0;
    let mut audio = AudioOut::new(project.clone(), seq, settings, range)?;
    audio.measure(settings, sources, &cancelled)?;

    progress.total.store(total_frames, Ordering::Relaxed);
    progress.set_status(format!("Exporting {total_frames} frames"));

    let stdin_w = stdin.clone();
    let _ = (|| -> Result<()> {
        while f < f1 {
            if cancelled() {
                return Err(ExportError::Cancelled);
            }
            let end = (f + batch).min(f1);

            // Render the batch of frames.
            let frames: Vec<Vec<u8>> = (f..end).map(|fi| pipe.frame(fi, sources).0).collect();

            // Write frames as rawvideo RGBA.
            {
                let mut stdin = stdin_w.lock().unwrap();
                for rgba in &frames {
                    if cancelled() {
                        return Err(ExportError::Cancelled);
                    }
                    stdin.write_all(rgba).map_err(|e| ExportError::Io(format!("ffmpeg write frame: {e}")))?;
                    written += 1;
                    progress.done.store(written, Ordering::Relaxed);
                }
            }

            // Pull audio for this batch's time span and write it as raw PCM.
            let start_sample = time_to_samples(f, rate, settings);
            let end_sample = time_to_samples(end, rate, settings);
            let n = (end_sample - start_sample).max(0);
            if n > 0 {
                let planar = audio.pull(n, sources).ok_or_else(|| ExportError::Encode("audio pull failed".into()))?;
                if !planar.is_empty() {
                    let mut stdin = stdin_w.lock().unwrap();
                    write_pcm_interleaved(&mut *stdin, &planar).map_err(|e| ExportError::Io(format!("ffmpeg write audio: {e}")))?;
                }
            }

            f = end;
        }

        // Close stdin so ffmpeg sees EOF and finishes encoding.
        drop(stdin_w);
        Ok(())
    })();

    let status = child.wait().map_err(|e| ExportError::Io(format!("ffmpeg wait: {e}")))?;
    let err_text = err_out.lock().unwrap().clone();

    if cancelled() {
        let _ = std::fs::remove_file(&settings.path);
        return Err(ExportError::Cancelled);
    }

    if !status.success() {
        return Err(ExportError::Encode(format!(
            "ffmpeg exited with code {:?}: {err_text}",
            status.code()
        )));
    }

    progress.finished.store(true, Ordering::Relaxed);
    let bytes = std::fs::metadata(&settings.path).map(|m| m.len()).unwrap_or(0);
    Ok(Report {
        path: settings.path.clone(),
        frames: total_frames,
        seconds: 0.0,
        bytes,
        render_fps: total_frames as f64,
        extra_files: Vec::new(),
    })
}

/// Convert a frame index at `rate` to the audio sample index at the output sample rate.
fn time_to_samples(frame: i64, rate: FrameRate, settings: &ExportSettings) -> i64 {
    let sr = settings.audio.sample_rate.unwrap_or(48_000) as i64;
    let ticks = rate.tick_of(frame);
    (ticks.0 * sr) / filmcraft_time::TICKS_PER_SECOND
}

/// Write interleaved PCM (f32 in [-1, 1], planar per channel) to ffmpeg's stdin as s16le.
fn write_pcm_interleaved(stdin: &mut impl Write, planar: &[Vec<f32>]) -> std::io::Result<()> {
    let ch = planar.len();
    let n = planar.first().map(|c| c.len()).unwrap_or(0);
    let mut buf = Vec::with_capacity(n * ch * 2);
    for i in 0..n {
        for c in 0..ch {
            let s = planar[c].get(i).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            let v = (s * 32767.0).round() as i16;
            buf.extend_from_slice(&v.to_le_bytes());
        }
    }
    stdin.write_all(&buf)
}

/// Build the ffmpeg command line for the given settings.
fn build_command(ffmpeg: &str, settings: &ExportSettings, w: u32, h: u32, rate: FrameRate) -> Result<Command> {
    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-hide_banner");
    cmd.arg("-y");

    let fps = rate.num as f64 / rate.den as f64;

    // Video input: rawvideo RGBA from stdin.
    cmd.arg("-f").arg("rawvideo");
    cmd.arg("-pix_fmt").arg("rgba");
    cmd.arg("-s").arg(format!("{w}x{h}"));
    cmd.arg("-framerate").arg(fps.to_string());
    cmd.arg("-i").arg("-");

    // Audio input: s16le from stdin.
    let sr = settings.audio.sample_rate.unwrap_or(48_000);
    let ch = settings.audio.channels.max(1);
    cmd.arg("-f").arg("s16le");
    cmd.arg("-ar").arg(sr.to_string());
    cmd.arg("-ac").arg(ch.to_string());
    cmd.arg("-i").arg("-");

    // Video codec and colour signalling.
    match settings.format {
        crate::Format::H264 | crate::Format::Hevc | crate::Format::Av1 => {
            let vcodec = match settings.format {
                crate::Format::H264 => "libx264",
                crate::Format::Hevc => "libx265",
                crate::Format::Av1 => "libaom-av1",
                _ => "libx264",
            };
            cmd.arg("-c:v").arg(vcodec);
            cmd.arg("-pix_fmt").arg("yuv420p");
            cmd.arg("-color_primaries").arg("bt709");
            cmd.arg("-color_trc").arg("bt709");
            cmd.arg("-colorspace").arg("bt709");
            match settings.bitrate_mode {
                crate::BitrateMode::Crf => {
                    cmd.arg("-crf").arg(settings.crf.to_string());
                }
                _ => {
                    cmd.arg("-b:v").arg(format!("{}k", settings.bitrate_kbps));
                    if let Some(max) = settings.max_bitrate_kbps {
                        cmd.arg("-maxrate").arg(format!("{}k", max));
                        cmd.arg("-bufsize").arg(format!("{}k", max.max(settings.bitrate_kbps)));
                    }
                }
            }
            cmd.arg("-preset").arg("medium");
        }
        crate::Format::Gif => {
            cmd.arg("-c:v").arg("gif");
        }
        _ => {
            cmd.arg("-c:v").arg("libx264");
            cmd.arg("-pix_fmt").arg("yuv420p");
            cmd.arg("-color_primaries").arg("bt709");
            cmd.arg("-color_trc").arg("bt709");
            cmd.arg("-colorspace").arg("bt709");
            cmd.arg("-b:v").arg(format!("{}k", settings.bitrate_kbps));
            cmd.arg("-preset").arg("medium");
        }
    }

    // Audio: AAC.
    cmd.arg("-c:a").arg("aac");
    cmd.arg("-b:a").arg(format!("{}k", settings.audio.bitrate_kbps.max(32)));

    // Fast start.
    cmd.arg("-movflags").arg("+faststart");

    // Metadata.
    if !settings.metadata.title.is_empty() {
        cmd.arg("-metadata").arg(format!("title={}", settings.metadata.title));
    }
    if !settings.metadata.comment.is_empty() {
        cmd.arg("-metadata").arg(format!("comment={}", settings.metadata.comment));
    }

    cmd.arg(&settings.path);

    Ok(cmd)
}

/// Encode a single still frame (PNG or JPEG) at sequence time `t`.
pub fn export_still(
    project: &Arc<Project>,
    seq: ItemId,
    t: Tick,
    path: &str,
    format: crate::Format,
    quality: u8,
) -> Result<()> {
    // Use the shared pipeline to render one frame at time t, then encode it directly.
    let settings = ExportSettings {
        format,
        path: path.to_string(),
        quality,
        ..Default::default()
    };
    let pipe = Pipeline::new(project.clone(), seq, &settings, false)?;
    let f = pipe.rate.frame_at(t);
    let (rgba, _floats) = pipe.frame(f, &EmptyProvider);
    let (w, h) = (pipe.w, pipe.h);
    let img = image::RgbaImage::from_raw(w, h, rgba).ok_or_else(|| ExportError::Encode("still frame".into()))?;

    match format {
        crate::Format::PngSequence | crate::Format::JpegSequence => {
            let file = std::fs::File::create(path).map_err(|e| ExportError::Io(e.to_string()))?;
            match format {
                crate::Format::PngSequence => {
                    let encoder = image::codecs::png::PngEncoder::new(file);
                    encoder.write_image(&img, w, h, image::ColorType::Rgba8.into()).map_err(|e| ExportError::Encode(e.to_string()))?;
                }
                crate::Format::JpegSequence => {
                    let mut buf = Vec::new();
                    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
                    encoder.encode_image(&img).map_err(|e| ExportError::Encode(e.to_string()))?;
                    use std::io::Write as _;
                    let mut file = std::fs::File::create(path).map_err(|e| ExportError::Io(e.to_string()))?;
                    file.write_all(&buf).map_err(|e| ExportError::Io(e.to_string()))?;
                }
                _ => {}
            }
        }
        _ => return Err(ExportError::Unsupported(format!("still format {format:?}"))),
    }

    Ok(())
}
