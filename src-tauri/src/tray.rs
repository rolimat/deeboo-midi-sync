//! Menu bar presentation: which icon frame and which title to show.

/// How long the dot stays bright after an input beat.
const BLINK_US: u64 = 120_000;

/// Figure space, as wide as a digit, to keep the title width steady.
const FIGURE_SPACE: char = '\u{2007}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayFrame {
    GreenOn,
    GreenDim,
    RedOn,
    RedDim,
}

/// Picks the icon frame. Colour follows the output (green when sending, red otherwise); the blink
/// follows the input (bright for 120 ms after a beat, else dim). Without input it is a steady red.
#[must_use]
pub fn tray_frame(in_ok: bool, out_ok: bool, last_beat_change_us: u64, now_us: u64) -> TrayFrame {
    if !in_ok {
        return TrayFrame::RedOn;
    }
    let bright = now_us.saturating_sub(last_beat_change_us) < BLINK_US;
    match (out_ok, bright) {
        (true, true) => TrayFrame::GreenOn,
        (true, false) => TrayFrame::GreenDim,
        (false, true) => TrayFrame::RedOn,
        (false, false) => TrayFrame::RedDim,
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

    const T0: u64 = 5_000_000;

    #[test]
    fn red_steady_without_input() {
        assert_eq!(tray_frame(false, true, T0, T0), TrayFrame::RedOn);
        assert_eq!(tray_frame(false, true, T0, T0 + 121_000), TrayFrame::RedOn);
        assert_eq!(
            tray_frame(false, false, T0, T0 + 5_000_000),
            TrayFrame::RedOn
        );
    }

    #[test]
    fn red_blinks_when_receiving_but_paused() {
        assert_eq!(tray_frame(true, false, T0, T0), TrayFrame::RedOn);
        assert_eq!(tray_frame(true, false, T0, T0 + 119_000), TrayFrame::RedOn);
        assert_eq!(tray_frame(true, false, T0, T0 + 121_000), TrayFrame::RedDim);
    }

    #[test]
    fn green_blinks_when_sending() {
        assert_eq!(tray_frame(true, true, T0, T0), TrayFrame::GreenOn);
        assert_eq!(tray_frame(true, true, T0, T0 + 119_000), TrayFrame::GreenOn);
        assert_eq!(
            tray_frame(true, true, T0, T0 + 121_000),
            TrayFrame::GreenDim
        );
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
