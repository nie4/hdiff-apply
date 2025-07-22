#![feature(once_cell_try)]

use std::{io::Write, path::PathBuf, time::Instant};

mod binary_version;
mod deletefiles;
mod error;
mod hdiffmap;
mod seven_util;
mod update_mgr;
mod utils;
mod verifier;

use binary_version::BinaryVersion;
use deletefiles::DeleteFiles;
use hdiffmap::HDiffMap;
use rand::{distr::Alphanumeric, Rng};
use rayon::iter::ParallelIterator;
use seven_util::SevenUtil;
use verifier::Verifier;

use crate::update_mgr::UpdateMgr;

type Error = error::Error;

pub const TEMP_DIR_NAME: &'static str = "hdiff-apply";

fn run() -> Result<(), Error> {
    println!("Preparing for update...");

    let game_path = utils::determine_game_path(std::env::args().nth(1))?;

    let client_version =
        BinaryVersion::parse(&game_path.join("StarRail_Data/StreamingAssets/BinaryVersion.bytes"))?;

    let mut update_mgr = UpdateMgr::new(
        utils::get_update_archives(&game_path)?,
        utils::get_and_create_temp_dir()?,
        client_version,
        game_path,
        utils::get_hpatchz()?,
    );
    update_mgr.prepare_update_info()?;

    let update_choice = {
        print!(
            "Proceed with this update sequence: {} (Y/n): ",
            update_mgr.show_update_sequence()
        );
        utils::wait_for_confirmation(true)
    };

    let integrity_check_choice = update_choice
        .then(|| {
            print!("Verify client integrity (Y/n): ");
            utils::wait_for_confirmation(true)
        })
        .unwrap_or(false);

    if update_choice {
        let now = Instant::now();
        update_mgr.update(integrity_check_choice)?;
        println!("\nFinished in {:.2?}", now.elapsed());
    }

    utils::wait_for_input();
    Ok(())
}

fn main() {
    #[cfg(target_os = "windows")]
    let _ = ansi_term::enable_ansi_support();

    utils::set_console_title();
    utils::clean_temp_hdiff_data();

    if let Err(err) = run() {
        utils::print_err(err);
        utils::wait_for_input();
    }
}
