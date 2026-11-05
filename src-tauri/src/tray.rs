//! Menu bar presentation: which icon frame and which title to show.

/// How long the signal dot stays bright after an input beat.
const BLINK_US: u64 = 120_000;

/// Figure space, as wide as a digit, to keep the title width steady.
const FIGURE_SPACE: char = '\u{2007}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayFrame {
    GreenOn,
    GreenDim,
    Red,
}

/// Picks the icon frame: red without signal, bright green for 120 ms after a beat, else dim green.
#[must_use]
pub fn tray_frame(
    in_ok: bool,
    _input_beats: u64,
    last_beat_change_us: u64,
    now_us: u64,
) -> TrayFrame {
    if !in_ok {
        TrayFrame::Red
    } else if now_us.saturating_sub(last_beat_change_us) < BLINK_US {
        TrayFrame::GreenOn
    } else {
        TrayFrame::GreenDim
    }
}

/// Formats the BPM with one decimal, left-padded with figure spaces to the width of "100.0".
#[must_use]
pub fn tray_title(bpm: Option<f64>) -> String {
    let Some(bpm) = bpm else {
        return "—".to_owned();
    };
    let text = format!("{bpm:.1}");
    let pad = 5usize.saturating_sub(text.chars().count());
    format!("{}{text}", FIGURE_SPACE.to_string().repeat(pad))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_without_signal() {
        assert_eq!(tray_frame(false, 10, 1_000, 1_000), TrayFrame::Red);
    }

    #[test]
    fn green_on_right_after_beat_then_dim() {
        let t0 = 5_000_000;
        assert_eq!(tray_frame(true, 3, t0, t0), TrayFrame::GreenOn);
        assert_eq!(tray_frame(true, 3, t0, t0 + 119_000), TrayFrame::GreenOn);
        assert_eq!(tray_frame(true, 3, t0, t0 + 121_000), TrayFrame::GreenDim);
    }

    #[test]
    fn title_formats_one_decimal_and_dash() {
        assert_eq!(tray_title(Some(124.04)), "124.0");
        assert_eq!(tray_title(Some(99.96)), "100.0");
        assert_eq!(tray_title(None), "—");
    }

    #[test]
    fn title_below_100_is_figure_space_padded() {
        assert_eq!(tray_title(Some(99.04)), "\u{2007}99.0");
        assert_eq!(tray_title(Some(8.0)), "\u{2007}\u{2007}8.0");
    }
}
