pub mod commands;
pub mod legado_json;
mod parser_registry;
pub mod rules;
pub mod types;

use std::path::PathBuf;
use tokio::sync::Mutex;

use crate::errors::Result;
use crate::fs_utils::app_data_dir;

pub use parser_registry::{ParserType, SourceFile};

pub struct BookSourceState {
    external_dirs: Mutex<Vec<PathBuf>>,
}

impl BookSourceState {
    pub fn new() -> Self {
        Self {
            external_dirs: Mutex::new(Vec::new()),
        }
    }

    pub async fn dirs(&self) -> Result<Vec<PathBuf>> {
        let mut dirs = vec![default_source_dir()?];
        dirs.extend(self.external_dirs.lock().await.iter().cloned());
        Ok(dirs)
    }

    pub async fn add_dir(&self, dir: PathBuf) {
        let mut dirs = self.external_dirs.lock().await;
        if !dirs.iter().any(|item| item == &dir) {
            dirs.push(dir);
        }
    }

    pub async fn remove_dir(&self, dir: &PathBuf) {
        let mut dirs = self.external_dirs.lock().await;
        dirs.retain(|item| item != dir);
    }
}

pub fn default_source_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("booksources"))
}
