use orcrustrator::compiler::OverridesManifest;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

/// Dynamic hot-reload manager watching `overrides.toml`
#[derive(Clone)]
pub struct HotReloadManager {
    overrides_path: PathBuf,
    last_modified: Arc<RwLock<Option<SystemTime>>>,
}

impl HotReloadManager {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            overrides_path: path.into(),
            last_modified: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if overrides.toml has been modified on disk and reload
    pub async fn check_and_reload(&self) -> Option<OverridesManifest> {
        if !self.overrides_path.exists() {
            return None;
        }

        if let Ok(metadata) = std::fs::metadata(&self.overrides_path) {
            if let Ok(modified) = metadata.modified() {
                let mut last = self.last_modified.write().await;
                if last.is_none() || *last != Some(modified) {
                    *last = Some(modified);
                    println!("[HotReload] overrides.toml change detected! Reloading manifests...");
                    return OverridesManifest::load_from_file(&self.overrides_path).ok();
                }
            }
        }

        None
    }
}
