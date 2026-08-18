//! Publishes `Status` to the state file the Omarchy panel watches, once per
//! change. A `FileView` on the other end has no debounce, so writing on every
//! poll tick would matter; writing only on change is what makes idle steady
//! state genuinely idle.

use std::path::PathBuf;
use std::sync::Arc;

use pbp2_common::Status;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct StatusWriter {
    path: PathBuf,
    last: Arc<Mutex<Option<String>>>,
}

impl StatusWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path, last: Arc::new(Mutex::new(None)) }
    }

    /// Writes `status` to the state file if, and only if, it differs from
    /// the last line written. Uses a temp file plus rename so a reader never
    /// observes a half-written line.
    pub async fn publish(&self, status: &Status) {
        let rendered = status.render();

        let mut last = self.last.lock().await;
        if last.as_deref() == Some(rendered.as_str()) {
            return;
        }

        if let Err(err) = self.write_atomic(&rendered).await {
            tracing::warn!(error = ?err, "failed to write status file");
            return;
        }

        *last = Some(rendered);
    }

    async fn write_atomic(&self, rendered: &str) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let tmp_path = self.path.with_extension(format!("json.tmp.{}", std::process::id()));
        tokio::fs::write(&tmp_path, rendered.as_bytes()).await?;
        tokio::fs::rename(&tmp_path, &self.path).await?;
        Ok(())
    }
}
