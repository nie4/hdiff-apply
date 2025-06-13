use std::{path::Path, time::Instant};

mod binary_version;
mod deletefiles;
mod error;
mod hdiffmap;
mod seven_util;
mod utils;
mod verifier;

use binary_version::BinaryVersion;
use clap::Parser;
use deletefiles::DeleteFiles;
use hdiffmap::HDiffMap;
use rand::{distr::Alphanumeric, Rng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use seven_util::SevenUtil;
use verifier::Verifier;

type Error = error::Error;

pub const TEMP_DIR_NAME: &'static str = "hdiff-apply";

#[derive(Parser, Debug)]
struct Args {
    #[arg()]
    game_path: Option<String>,
    #[arg(long)]
    skip_version_check: bool,
}

fn run() -> Result<(), Error> {
    #[cfg(target_os = "windows")]
    let _ = ansi_term::enable_ansi_support();

    utils::set_console_title()?;
    utils::clean_temp_hdiff_data()?;

    let args = Args::parse();

    if args.skip_version_check {
        println!("- `--skip-version-check` is deprecated and has no effect. Make sure your archive remains unextracted")
    }

    let temp_dir_path = utils::get_and_create_temp_dir()?;
    let hpatchz_path = utils::get_hpatchz()?;
    let game_path = utils::determine_game_path(args.game_path)?;
    let update_archives_paths = utils::get_update_archives(&game_path)?;

    println!("Preparing for update...");

    // <(hdiff_version, temp_path, archive_path)>
    let mut updates_big_vec: Vec<_> = update_archives_paths
        .into_par_iter()
        .map(|update_archive_path| {
            let rnd_name: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(5)
                .map(char::from)
                .collect();

            let temp_path = temp_dir_path.join(format!("hdiff_{}", rnd_name));

            SevenUtil::inst().extract_specific_files_to(
                &update_archive_path,
                &[
                    "StarRail_Data\\StreamingAssets\\BinaryVersion.bytes",
                    "hdiffmap.json",
                    "deletefiles.txt",
                ],
                &temp_path,
            )?;

            let hdiff_version = BinaryVersion::parse(&temp_path.join("BinaryVersion.bytes"))?;

            Ok((hdiff_version, temp_path, update_archive_path.to_path_buf()))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    updates_big_vec.sort_by(|a, b| a.0.cmp(&b.0));

    let client_version = BinaryVersion::parse(
        &game_path.join("StarRail_Data\\StreamingAssets\\BinaryVersion.bytes"),
    )?;

    let mut start_index = None;
    let mut sequence = String::new();

    for (i, (hdiff_version, _, _)) in updates_big_vec.iter().enumerate() {
        if start_index.is_none() {
            if !utils::verify_hdiff_version(&client_version, hdiff_version) {
                continue;
            }
            start_index = Some(i);

            sequence.push_str(&format!("{}", client_version.to_string()));
            sequence.push_str(&format!(" → {}", hdiff_version.patch_version));

            continue;
        }

        if utils::verify_hdiff_version(&client_version, hdiff_version) {
            return Err(Error::InvalidHdiffVersion(
                client_version.to_string(),
                hdiff_version.to_string(),
            ));
        }

        sequence.push_str(&format!(" → {}", hdiff_version.patch_version));
    }

    if start_index.is_none() {
        let last_hdiff = updates_big_vec
            .last()
            .map(|(v, _, _)| v.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        return Err(Error::InvalidHdiffVersion(
            client_version.to_string(),
            last_hdiff,
        ));
    }

    let update_choice = {
        print!("Proceed with this update sequence: {} (Y/n): ", sequence);
        utils::wait_for_confirmation(true)
    };

    let integrity_check_choice = if update_choice {
        print!("Verify client integrity (Y/n): ");
        utils::wait_for_confirmation(true)
    } else {
        false
    };

    let now = Instant::now();

    if update_choice {
        if let Some(index) = start_index {
            let updates_len = updates_big_vec.len() - index;

            for (i, (hdiff_version, temp_path, update_archive_path)) in
                updates_big_vec.iter().skip(index).enumerate()
            {
                let hdiffmap_path = temp_path.join("hdiffmap.json");
                let deletefiles_path = temp_path.join("deletefiles.txt");

                println!("\n-- Update {} of {}", i + 1, updates_len);

                if integrity_check_choice {
                    println!("Verifying client integrity");

                    let verify_client = Verifier::new(game_path.as_path(), &hdiffmap_path);
                    verify_client.by_file_size()?;
                    verify_client.by_md5()?;
                }

                let archive_name = update_archive_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("hdiff");

                println!("Extracting {}", archive_name);
                SevenUtil::inst().extract_hdiff_to(&update_archive_path, &game_path)?;

                run_updater(&game_path, &hpatchz_path, &hdiffmap_path, &deletefiles_path)?;

                println!("Updated to {}", hdiff_version.to_string())
            }
        }
    }

    println!("\nFinished in {:.2?}", now.elapsed());
    utils::wait_for_input();
    Ok(())
}

fn run_updater(
    game_path: &Path,
    hpatchz_path: &Path,
    hdiffmap_path: &Path,
    deletefiles_path: &Path,
) -> Result<(), Error> {
    println!("Running updater");

    let mut delete_files = DeleteFiles::new(&game_path, &deletefiles_path);
    if let Err(e) = delete_files.remove() {
        utils::print_err(&e.to_string());
    }

    let mut hdiff_map = HDiffMap::new(&game_path, &hpatchz_path, &hdiffmap_path);
    if let Err(e) = hdiff_map.patch() {
        utils::print_err(&e.to_string());
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        utils::print_err(&e.to_string());
        utils::wait_for_input()
    }
}
