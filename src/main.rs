#![feature(once_cell_try, try_blocks)]

use std::{io, time::Instant};

use anyhow::Result;
use crossterm::{execute, terminal::SetTitle};

use crate::{update::manager::UpdateMgr, utils::{hpatchz::HPatchZ, seven_zip::SevenZip}};

mod types;
mod update;
mod utils;

pub const TEMP_DIR_NAME: &str = "hdiff-apply";

fn run() -> Result<()> {
    let game_path = utils::determine_game_path(std::env::args().nth(1))?;

    let mut update_mgr = UpdateMgr::new(game_path)?;
    update_mgr.prepare_updates()?;

    let update_message = format!(
        "Proceed with this update sequence: {}",
        update_mgr.update_sequence()
    );

    let do_update = utils::confirm(&update_message, true);
    let do_integrity_check = do_update && utils::confirm("Verify client integrity", true);

    if do_update {
        let now = Instant::now();
        update_mgr.update(do_integrity_check)?;
        println!("\nFinished in {:.2?}", now.elapsed());
    }

    utils::wait_for_input();
    Ok(())
}

fn main() {
    utils::clean_temp_hdiff_data();

    println!("Preparing update... this may take a few seconds");

    let result: Result<()> = try {
        execute!(
            io::stdout(),
            SetTitle(format!(
                "{} v{} | Made by nie",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
        )?;

        utils::get_or_create_temp_dir()?;

        // Just to throw error early if any occurs
        HPatchZ::instance()?;
        SevenZip::instance()?;

        run()?
    };

    if let Err(e) = result {
        utils::print_err(e);
        utils::wait_for_input();
    }

    utils::clean_temp_hdiff_data();
}
