use crate::models::ConversionJob;
use anyhow::{Context, Result};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

pub fn convert(
    app: &AppHandle,
    ffmpeg: &str,
    input: &Path,
    output_dir: &Path,
    media_id: &str,
    duration_secs: f64,
    quality: &str,
) -> Result<PathBuf> {
    fs::create_dir_all(output_dir)?;
    let output = output_dir.join(format!("{media_id}.mp4"));
    let crf = match quality {
        "compact" => "26",
        "quality" => "19",
        _ => "22",
    };
    let mut child = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-progress",
            "pipe:1",
            "-nostats",
            "-y",
            "-i",
        ])
        .arg(input)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            crf,
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-movflags",
            "+faststart",
        ])
        .arg(&output)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Could not start {ffmpeg}"))?;
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(raw) = line.strip_prefix("out_time_us=") {
                if let Ok(micros) = raw.parse::<f64>() {
                    let progress = if duration_secs > 0.0 {
                        (micros / 1_000_000.0 / duration_secs).clamp(0.0, 0.99)
                    } else {
                        0.0
                    };
                    let _ = app.emit(
                        "conversion-progress",
                        ConversionJob {
                            media_id: media_id.into(),
                            status: "running".into(),
                            progress,
                            output_path: None,
                            error: None,
                        },
                    );
                }
            }
        }
    }
    let status = child.wait()?;
    if !status.success() {
        let _ = fs::remove_file(&output);
        anyhow::bail!("ffmpeg conversion failed with {status}");
    }
    Ok(output)
}
