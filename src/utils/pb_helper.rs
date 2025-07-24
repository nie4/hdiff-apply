use indicatif::{ProgressBar, ProgressStyle};

static PROGRESS_TEMPLATE: &str = "{spinner:.green} [{elapsed}] [{bar:35.cyan/blue}] {pos}/{len}";
static PROGRESS_CHARS: &str = "#>-";

// Helper function for indicatif crate that handles global pb style
pub fn create_progress_bar(len: usize) -> ProgressBar {
    let pb = ProgressBar::new(len as u64);
    pb.set_style(
        ProgressStyle::with_template(PROGRESS_TEMPLATE)
            .unwrap()
            .progress_chars(PROGRESS_CHARS),
    );
    pb
}
