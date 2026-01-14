use std::{
    collections::HashSet,
    io::{self, Write},
};

use anyhow::{Context, Result, anyhow, bail};
use crossterm::{
    ExecutableCommand, cursor,
    terminal::{self, ClearType},
};
use indicatif::{ProgressBar, ProgressStyle};

use crate::{Args, patchers::PatchManager, update_package::UpdatePackage};

pub fn run(args: &Args) -> Result<()> {
    if !args.game_path.exists() {
        return Err(anyhow!("Game path doesn't exist"));
    }

    let archives_path = args.archives_path.as_ref().unwrap_or(&args.game_path);

    print_banner();

    let archives = UpdatePackage::find(&archives_path)?;
    if archives.is_empty() {
        bail!("Didn't find any archives - make sure you're in the correct directory.")
    }

    let selected_archives = select_archives(archives)?;
    let total_count = selected_archives.len();

    println!("-------------------------------");

    for (idx, package) in selected_archives.into_iter().enumerate() {
        let current = idx + 1;
        println!("[{}/{}] Processing: {}", current, total_count, package.name);

        print!("  Extracting archive... ");
        io::stdout().flush()?;

        if let Err(e) = package.extract(&args.game_path) {
            println!("FAILED");
            return Err(e);
        }
        println!("OK");

        let patcher = PatchManager::new(&args.game_path);
        let patch_bar = ProgressBar::new(0);
        patch_bar.set_style(
            ProgressStyle::default_bar()
                .template("  {msg:<20} [{bar:40.cyan/blue}] {pos:>4}/{len:4} ({percent}%)")?
                .progress_chars("##-"),
        );

        if let Err(e) = patcher.patch(&patch_bar) {
            patch_bar.finish_and_clear();
            return Err(e.context("Patch error - game files remain unchanged!"));
        }

        patch_bar.finish_and_clear();
        println!("  Patching complete using {}", patcher.patcher_name());
        println!();
    }

    println!("-------------------------------");
    println!("All {} updates completed successfully!", total_count);

    Ok(())
}

fn print_banner() {
    println!(
        "{} v{} | Made by nie\n-------------------------------",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
}

fn select_archives(archives: Vec<UpdatePackage>) -> Result<Vec<UpdatePackage>> {
    if archives.len() == 1 {
        return select_single_archive(archives);
    }

    println!("Found {} update packages:", archives.len());

    let max_name_width = archives.iter().map(|a| a.name.len()).max().unwrap_or(0);
    for (i, archive) in archives.iter().enumerate() {
        println!(
            "  {}. {:<width$} | {}",
            i + 1,
            archive.name,
            archive.size.display(),
            width = max_name_width
        );
    }
    println!("");

    loop {
        println!("Enter the order in which to apply them (e.g. `1 2` or `2 1`)");
        let input = read_line()?;

        clear_lines(1)?;

        let selected_archives = match parse_order(&input, &archives) {
            Ok(archives) => archives,
            Err(e) => {
                clear_lines(1)?;
                println!("\x1b[41m{}\x1b[m", e);
                continue;
            }
        };

        println!("\nSelected order:");
        for (i, archive) in selected_archives.iter().enumerate() {
            println!("  {}. {}", i + 1, archive.name);
        }

        if confirm_order(&selected_archives)? {
            return Ok(selected_archives);
        } else {
            clear_lines(selected_archives.len() + 5)?;
        }
    }
}

fn select_single_archive(archives: Vec<UpdatePackage>) -> Result<Vec<UpdatePackage>> {
    println!("Update package found:");
    println!("  Name: {}", archives[0].name);
    println!("  Size: {}", archives[0].size.display());

    print!("\nPress 'c' to confirm or 'q' to quit: ");
    let input = read_line()?;

    match input.trim().to_lowercase().as_str() {
        s if s.contains('c') => Ok(archives),
        _ => bail!("Operation cancelled"),
    }
}

fn confirm_order(archives: &Vec<UpdatePackage>) -> Result<bool> {
    print!("\nPress 'c' to confirm or 'e' to edit: ");
    let input = read_line()?;

    match input.trim().to_lowercase().as_str() {
        "c" => Ok(true),
        "e" => Ok(false),
        _ => {
            println!("Invalid input. Please enter 'c' or 'e'.");
            confirm_order(archives)
        }
    }
}

fn parse_order(input: &str, archives: &[UpdatePackage]) -> Result<Vec<UpdatePackage>> {
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
        selected.push(archives[idx - 1].clone());
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
