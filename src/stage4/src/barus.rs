use std::fmt::Write;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};

pub fn create_barus() -> ProgressBar {
    let pb = ProgressBar::new(1);
    pb.set_style(ProgressStyle::with_template("{prefix:.bold} {spinner:.green} {msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));
    pb
}
