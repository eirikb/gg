use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};
use std::fmt::Write;

// No trim - stage3 and the ps1 can only tell set from empty, and all three
// have to agree on what "set" means
fn hidden(raw: Option<String>) -> bool {
    raw.is_some_and(|v| !v.is_empty())
}

/// Set GG_HIDE_DOWNLOAD_PROGRESS when something parses gg's output (#309)
pub fn progress_hidden() -> bool {
    hidden(std::env::var("GG_HIDE_DOWNLOAD_PROGRESS").ok())
}

pub fn create_multi_barus() -> MultiProgress {
    create_multi_barus_with(progress_hidden())
}

fn create_multi_barus_with(hidden: bool) -> MultiProgress {
    if hidden {
        // A MultiProgress hands its own draw target to every bar it takes,
        // so create_barus on its own is not enough
        MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
    } else {
        MultiProgress::new()
    }
}

pub fn create_barus() -> ProgressBar {
    create_barus_with(progress_hidden())
}

fn create_barus_with(hidden: bool) -> ProgressBar {
    if hidden {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(1);
    pb.set_style(ProgressStyle::with_template("{prefix:.bold} {spinner:.green} {msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));
    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hidden_needs_a_real_value() {
        assert!(hidden(Some("1".to_string())));
        assert!(hidden(Some("yes please".to_string())));
        assert!(!hidden(None));
        // GG_HIDE_DOWNLOAD_PROGRESS= reads as "not set", same as unset
        assert!(!hidden(Some("".to_string())));
        // ...but whitespace is a value, because that is all stage3 can see
        assert!(hidden(Some("  ".to_string())));
    }

    #[test]
    fn test_hidden_bar_stays_hidden_inside_a_multi() {
        assert!(create_barus_with(true).is_hidden());
        let m = create_multi_barus_with(true);
        assert!(m.add(ProgressBar::new_spinner()).is_hidden());
        assert!(m.insert(0, create_barus_with(true)).is_hidden());
        // No point asserting the visible side - indicatif calls a bar hidden
        // when stderr is not a terminal, and under cargo test it is not
    }
}
