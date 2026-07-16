use std::{
    collections::HashSet,
    env,
    fs::{self},
    io::{self, Write},
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use crossterm::{
    ExecutableCommand, cursor,
    terminal::{self, ClearType},
};
use indicatif::{ProgressBar, ProgressStyle};

use crate::{patchers::PatchManager, update_package::UpdatePackage};

pub const RESET: &'static str = "\x1b[0m";
pub const WHITE: &'static str = "\x1b[1;87m";
pub const YELLOW: &'static str = "\x1b[33m";
pub const RED: &'static str = "\x1b[1;31m";
pub const GREEN: &'static str = "\x1b[1;32m";

pub fn print_banner() {
    println!(
        "{} v{} : Made by nie\n",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
}

pub fn run(game_path: &Path, archives_path: &Path) -> Result<()> {
    if !game_path.exists()
        || !archives_path.exists()
        || !game_path.is_dir()
        || !archives_path.is_dir()
    {
        bail!("'{}' is not a valid directory", game_path.display());
    }

    let archives = UpdatePackage::find(archives_path)?;
    if archives.is_empty() {
        bail!("Didn't find any archives in '{}'", archives_path.display())
    }

    let selected_indices = select_archives(&archives)?;
    let total_count = selected_indices.len();

    println!("-------------------------------");

    for (i, idx) in selected_indices.into_iter().enumerate() {
        let current = i + 1;
        let package = &archives[idx];

        println!("[{}/{}] Processing: {}", current, total_count, package.name);

        print!("  Extracting archive... ");
        io::stdout().flush()?;

        let temp_extract = HaTemp::new(game_path.join(".ha-extracted"))?;
        package.extract(&temp_extract)?;
        println!("{GREEN}OK{RESET}");

        run_patcher(game_path, &temp_extract)?;
        merge_into_game(&temp_extract, game_path)?;
    }

    println!("{WHITE}All {total_count} updates completed successfully!{RESET}");

    Ok(())
}

fn merge_into_game(from: &Path, to: &Path) -> Result<()> {
    fn is_patch_metadata(name: &str) -> bool {
        matches!(
            name,
            "hdifffiles.txt" | "hdiffmap.json" | "deletefiles.txt" | "ldiff"
        ) || name.starts_with("manifest")
    }

    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();

        if is_patch_metadata(&name.to_string_lossy()) {
            continue;
        }

        let src = entry.path();
        let dst = to.join(&name);

        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&dst)?;
            merge_into_game(&src, &dst)?;
        } else {
            fs::rename(&src, &dst)?;
        }
    }

    Ok(())
}

fn run_patcher(game_path: &Path, patch_path: &Path) -> Result<()> {
    let patcher = PatchManager::new(game_path, patch_path)?;

    let patch_bar = ProgressBar::new(0);
    patch_bar.set_style(
        ProgressStyle::default_bar()
            .template("  {msg:<20} [{bar:40.cyan/blue}] {pos:>4}/{len:4} ({percent}%)")?
            .progress_chars("##-"),
    );

    let result = patcher.patch(&patch_bar);

    patch_bar.finish_and_clear();

    result.context("Patch failed - game files remain unchanged!")?;

    println!("  Patching complete using {}", patcher.patcher_name());
    println!();

    Ok(())
}

fn select_archives(archives: &[UpdatePackage]) -> Result<Vec<usize>> {
    if archives.len() == 1 {
        return Ok(vec![0]);
    }

    println!("{WHITE}Found {} update packages{RESET}:", archives.len());

    let max_name_width = archives.iter().map(|a| a.name.len()).max().unwrap_or(0);
    for (i, archive) in archives.iter().enumerate() {
        println!(
            "  [{}] {:<width$} ({})",
            i + 1,
            archive.name,
            archive.size,
            width = max_name_width
        );
    }
    println!();

    let mut extra_lines = 0;
    loop {
        println!("{WHITE}Enter update order (e.g. `1 2`){RESET}");
        print!("> ");
        let input = read_line()?;
        clear_lines(1)?;

        let selected_indices = match parse_order(&input, &archives) {
            Ok(archives) => archives,
            Err(e) => {
                clear_lines(1 + extra_lines)?;
                println!("{YELLOW}{}{RESET}", e);
                extra_lines = 1;
                continue;
            }
        };

        clear_lines(2 + extra_lines)?;
        println!("\n{WHITE}Selected order{RESET}:");
        for (i, idx) in selected_indices.iter().enumerate() {
            println!("  {}. {}", i + 1, &archives[*idx].name);
        }

        println!();
        if confirm_order()? {
            return Ok(selected_indices);
        } else {
            clear_lines(selected_indices.len() + 3)?;
            extra_lines = 0;
        }
    }
}

fn confirm_order() -> Result<bool> {
    loop {
        print!("{WHITE}Proceed [Y/n]{RESET} ");

        match read_line()?.trim().to_ascii_lowercase().as_str() {
            "" | "y" => return Ok(true),
            "n" => return Ok(false),
            _ => continue,
        }
    }
}

fn parse_order(input: &str, archives: &[UpdatePackage]) -> Result<Vec<usize>> {
    let indices: Vec<usize> = input
        .trim()
        .split_whitespace()
        .map(|s| s.parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .context("Invalid input - please enter numbers separated by spaces")?;

    if indices.is_empty() {
        bail!("No archives selected");
    }

    let mut seen = HashSet::new();
    for &idx in &indices {
        if !seen.insert(idx) {
            bail!(
                "Duplicate archive number: {}. Each archive can only be selected once",
                idx
            );
        }
    }

    let mut selected = Vec::new();
    for &idx in &indices {
        if idx < 1 || idx > archives.len() {
            bail!(
                "Invalid archive number: {}. Must be between 1 and {}",
                idx,
                archives.len()
            );
        }
        selected.push(idx - 1);
    }

    Ok(selected)
}

fn clear_lines(n: usize) -> Result<()> {
    for _ in 0..n {
        io::stdout()
            .execute(cursor::MoveUp(1))?
            .execute(terminal::Clear(ClearType::CurrentLine))?;
    }
    Ok(())
}

fn read_line() -> Result<String> {
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input)
}

pub struct HaTemp(PathBuf);

impl HaTemp {
    pub fn new(path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
}

impl Drop for HaTemp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl Deref for HaTemp {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
