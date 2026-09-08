//! Local media generation; no Telegram operations happen here.

use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use tdx::prelude::*;
use tokio::process::Command;

use crate::PHOTO_COUNT;

/// Creates a sparse local document; the caption distinguishes live-test messages.
pub fn document(root: &Path, name: &str, size: u64) -> Result<types::inputMessageDocument> {
  let path = root.join(name);
  let file = File::create(&path).context("failed to create a live-test document")?;
  file.set_len(size)?;

  let path = path.to_str().context("live-test path isn't valid UTF-8")?;
  Ok(content::document(file::local(path), None, "tdx live test".text().into()))
}

pub async fn photos(root: &Path) -> Result<Vec<enums::InputMessageContent>> {
  let mut contents = Vec::with_capacity(PHOTO_COUNT);

  for index in 0..PHOTO_COUNT {
    let path = root.join(format!("{index}.jpg"));
    let hue = format!("hue=h={}", index * 30);
    let status = Command::new("ffmpeg") //.
      .kill_on_drop(true)
      .args(["-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=2048x2048:rate=1:duration=1"])
      .args(["-vf", &hue, "-frames:v", "1"])
      .arg(&path)
      .status()
      .await
      .context("failed to run FFmpeg")?;
    ensure!(status.success(), "FFmpeg failed for photo {index}");
    let path = path.to_str().context("live-test path isn't valid UTF-8")?;
    contents.push(content::photo(file::local(path), [2048, 2048], "tdx live test".text().into()).into());
  }

  Ok(contents)
}
