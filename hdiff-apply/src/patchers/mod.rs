use std::path::{Path, PathBuf};

use anyhow::Result;
use indicatif::ProgressBar;

use crate::patchers::{custom_hdiff::CustomHdiff, hdiff::Hdiff, ldiff::Ldiff};

mod custom_hdiff;
mod hdiff;
mod ldiff;

pub trait Patcher {
    fn patch(&self, game_path: &Path, progress: Option<&ProgressBar>) -> Result<()>;
    fn name(&self) -> &'static str;
}

pub struct PatchManager {
    game_path: PathBuf,
    patcher: Box<dyn Patcher>,
}

impl PatchManager {
    pub fn new(game_path: &Path) -> Self {
        let patcher = Self::create_patcher(&game_path);
        Self {
            game_path: game_path.to_path_buf(),
            patcher,
        }
    }

    pub fn create_patcher(game_path: &Path) -> Box<dyn Patcher> {
        if game_path.join("manifest").exists() {
            Box::new(Ldiff)
        } else if game_path.join("GameAssembly.dll.hdiff").exists() {
            Box::new(CustomHdiff)
        } else {
            Box::new(Hdiff)
        }
    }

    pub fn patch(&self, progress: Option<&ProgressBar>) -> Result<()> {
        self.patcher.patch(&self.game_path, progress)
    }

    pub fn patcher_name(&self) -> &'static str {
        self.patcher.name()
    }
}
