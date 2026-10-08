//! `--fidelity-bench`: plays the motion capture's scripted fight once at each
//! graphics fidelity step in one window, saves the same frame from each, and
//! reports how long frames took. Vertical sync is off and every frame waits
//! for the GPU to finish the one before, so a frame's time includes its GPU
//! work rather than a display's refresh.
use crate::fidelity::Fidelity;

/// Frames per step: the motion capture's 15 seconds at 60 frames a second.
pub const FRAMES: u32 = 900;
/// Frames left out of the timing at the start of each step.
pub const WARMUP: u32 = 60;
/// The frame saved from each step: a fire vessel bursting beside the hero.
pub const SHOT: u32 = 720;

#[derive(Default)]
pub struct Bench {
    times: [Vec<f64>; 4],
    last: Option<f64>,
}

impl Bench {
    /// The step being played on the given frame of the whole bench.
    pub fn fidelity(frame: u32) -> Fidelity {
        Fidelity::ALL[(frame / FRAMES).min(3) as usize]
    }
    /// The frame within the current step.
    pub fn step_frame(frame: u32) -> u32 {
        frame % FRAMES
    }
    pub fn finished(frame: u32) -> bool {
        frame >= FRAMES * 4
    }
    /// Notes a frame starting at `now` (milliseconds, after waiting for the
    /// GPU). The time since the last one is kept unless it covered warm-up or
    /// the saved frame's readback.
    pub fn mark(&mut self, frame: u32, now: f64) {
        if let (Some(last), Some(previous)) = (self.last, frame.checked_sub(1)) {
            // The interval ending now belongs to the frame before this one.
            let measured = Self::step_frame(previous);
            if measured >= WARMUP && measured != SHOT && measured != SHOT + 1 {
                self.times[(previous / FRAMES).min(3) as usize].push(now - last);
            }
        }
        self.last = Some(now);
    }
    /// A table of each step's frame times.
    pub fn report(&self) -> String {
        let mut lines =
            vec!["fidelity  frames  mean ms  median ms  p95 ms  p99 ms  mean fps".to_string()];
        for (fidelity, times) in Fidelity::ALL.into_iter().zip(&self.times) {
            let s = Summary::of(times);
            lines.push(format!(
                "{:<8}  {:>6}  {:>7.2}  {:>9.2}  {:>6.2}  {:>6.2}  {:>8.0}",
                fidelity.name(),
                times.len(),
                s.mean,
                s.median,
                s.p95,
                s.p99,
                if s.mean > 0. { 1000. / s.mean } else { 0. },
            ));
        }
        lines.join("\n")
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Summary {
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
}
impl Summary {
    pub fn of(times: &[f64]) -> Self {
        if times.is_empty() {
            return Self::default();
        }
        let mut sorted = times.to_vec();
        sorted.sort_by(f64::total_cmp);
        let at =
            |share: f64| sorted[((sorted.len() as f64 * share) as usize).min(sorted.len() - 1)];
        Self {
            mean: sorted.iter().sum::<f64>() / sorted.len() as f64,
            median: at(0.5),
            p95: at(0.95),
            p99: at(0.99),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_step_plays_the_whole_script_in_order() {
        assert_eq!(Bench::fidelity(0), Fidelity::Low);
        assert_eq!(Bench::fidelity(FRAMES - 1), Fidelity::Low);
        assert_eq!(Bench::fidelity(FRAMES), Fidelity::Medium);
        assert_eq!(Bench::fidelity(FRAMES * 3 + 5), Fidelity::Ultra);
        assert_eq!(Bench::step_frame(FRAMES * 2 + SHOT), SHOT);
        assert!(!Bench::finished(FRAMES * 4 - 1) && Bench::finished(FRAMES * 4));
    }

    #[test]
    fn warm_up_and_the_saved_frame_are_left_out() {
        let mut bench = Bench::default();
        for frame in 0..=FRAMES * 4 {
            bench.mark(frame, frame as f64 * 2.);
        }
        for times in &bench.times {
            // Each step keeps every frame after warm-up but the saved one
            // and the one after it.
            assert_eq!(times.len() as u32, FRAMES - WARMUP - 2);
            assert!(times.iter().all(|t| *t == 2.));
        }
        assert!(bench.report().contains("Ultra"));
    }

    #[test]
    fn summaries_read_the_sorted_times() {
        let times: Vec<f64> = (1..=100).rev().map(f64::from).collect();
        let s = Summary::of(&times);
        assert_eq!((s.mean, s.median, s.p95, s.p99), (50.5, 51., 96., 100.));
        assert_eq!(Summary::of(&[]), Summary::default());
    }
}
