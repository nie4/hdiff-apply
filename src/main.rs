#![feature(once_cell_try)]

use std::time::Instant;

use crate::{error::AppError, update::manager::UpdateMgr};

mod error;
mod update;
mod utils;

pub const TEMP_DIR_NAME: &str = "hdiff-apply";

fn run() -> Result<(), AppError> {
    let game_path = utils::determine_game_path(std::env::args().nth(1))?;

    let mut update_mgr = UpdateMgr::new(game_path)?;
    update_mgr.prepare_update_info()?;

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
    println!("Preparing for update...");
    utils::set_console_title();
    utils::clean_temp_hdiff_data();
    
    if let Err(e) = run() {
        utils::print_err(e);
        utils::wait_for_input();
    }
}
