//! Statistics for MIDI clock inter-pulse intervals.

/// Summary of a series of intervals, all in microseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub count: usize,
    pub mean_us: f64,
    /// Population standard deviation (divides by n).
    pub stddev_us: f64,
    /// 99th percentile (nearest-rank) of |interval - mean|.
    pub p99_abs_dev_us: f64,
}

/// Summarizes `intervals_us`; `None` if there are fewer than 2 intervals.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn summarize(intervals_us: &[f64]) -> Option<Stats> {
    let n = intervals_us.len();
    if n < 2 {
        return None;
    }
    let nf = n as f64;
    let mean = intervals_us.iter().sum::<f64>() / nf;
    let variance = intervals_us.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / nf;
    let mut devs: Vec<f64> = intervals_us.iter().map(|x| (x - mean).abs()).collect();
    devs.sort_by(f64::total_cmp);
    let rank = (0.99 * nf).ceil() as usize; // nearest-rank, 1-based
    let p99 = devs[rank.clamp(1, n) - 1];
    Some(Stats {
        count: n,
        mean_us: mean,
        stddev_us: variance.sqrt(),
        p99_abs_dev_us: p99,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_intervals_have_zero_jitter() {
        let s = summarize(&[20000.0; 100]).unwrap();
        assert_eq!(s.count, 100);
        assert!((s.mean_us - 20000.0).abs() < 1e-9);
        assert!(s.stddev_us.abs() < 1e-9);
        assert!(s.p99_abs_dev_us.abs() < 1e-9);
    }

    #[test]
    fn known_jitter() {
        let v: Vec<f64> = (0..100)
            .map(|i| if i % 2 == 0 { 19000.0 } else { 21000.0 })
            .collect();
        let s = summarize(&v).unwrap();
        assert!((s.mean_us - 20000.0).abs() < 1e-9);
        assert!((s.stddev_us - 1000.0).abs() < 1e-9);
        assert!((s.p99_abs_dev_us - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn too_few_intervals_is_none() {
        assert!(summarize(&[]).is_none());
        assert!(summarize(&[20000.0]).is_none());
    }
}
