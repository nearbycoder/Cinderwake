//! `--fidelity-bench`: times frames at each graphics fidelity step and saves
//! the same frame from each, in one window.
//!
//! Timing plays the motion capture's scripted fight twice while the steps
//! take turns in blocks of a few frames, so a burst of load from elsewhere on
//! the machine (or another program using the GPU) lands on every step alike
//! rather than on whichever step happened to be running. Vertical sync is off
//! and each frame waits for the GPU to finish the one before, so a frame's
//! time includes its GPU work rather than a display's refresh.
//!
//! Then the script is replayed once per step up to the saved frame, so each
//! picture shows that step's own particles and lighting at the same moment.
use crate::fidelity::Fidelity;

/// Frames in the motion capture's script: 15 seconds at 60 frames a second.
pub const SCRIPT: u32 = 900;
/// Frames timed: the script, twice.
pub const TIMED: u32 = SCRIPT * 2;
/// Frames each step runs before the next takes its turn.
pub const BLOCK: u32 = 30;
/// Frames left out at the start of each block, while the step settles in.
pub const SETTLE: u32 = 5;
/// Frames left out at the start of the script.
pub const WARMUP: u32 = 60;
/// The frame saved from each step: a fire vessel bursting beside the hero.
pub const SHOT: u32 = 720;

/// What a frame of the bench is for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Timed, at this step, this many frames into the script.
    Timing(Fidelity, u32),
    /// Replaying the script at this step toward its saved frame.
    Picture(Fidelity, u32),
    Finished,
}

#[derive(Default)]
pub struct Bench {
    times: [Vec<f64>; 4],
    last: Option<f64>,
}

impl Bench {
    pub fn phase(frame: u32) -> Phase {
        if frame < TIMED {
            let step = Fidelity::ALL[((frame / BLOCK) % 4) as usize];
            return Phase::Timing(step, frame % SCRIPT);
        }
        let replay = frame - TIMED;
        let pass = replay / (SHOT + 1);
        if pass >= 4 {
            return Phase::Finished;
        }
        Phase::Picture(Fidelity::ALL[pass as usize], replay % (SHOT + 1))
    }
    /// The frame within the script, which restarts at 0.
    pub fn script_frame(frame: u32) -> u32 {
        match Self::phase(frame) {
            Phase::Timing(_, at) | Phase::Picture(_, at) => at,
            Phase::Finished => 0,
        }
    }
    pub fn fidelity(frame: u32) -> Fidelity {
        match Self::phase(frame) {
            Phase::Timing(f, _) | Phase::Picture(f, _) => f,
            Phase::Finished => Fidelity::High,
        }
    }
    /// Whether this frame is the one to save.
    pub fn shot(frame: u32) -> bool {
        matches!(Self::phase(frame), Phase::Picture(_, SHOT))
    }
    /// Notes a frame starting at `now` (milliseconds, after waiting for the
    /// GPU). The time since the last one belongs to the frame before, and is
    /// kept if that frame was timed, past warm-up, and settled in its block.
    pub fn mark(&mut self, frame: u32, now: f64) {
        if let (Some(last), Some(previous)) = (self.last, frame.checked_sub(1)) {
            if let Phase::Timing(step, at) = Self::phase(previous) {
                if at >= WARMUP && previous % BLOCK >= SETTLE {
                    self.times[step.index() as usize].push(now - last);
                }
            }
        }
        self.last = Some(now);
    }
    /// A table of each step's frame times.
    pub fn report(&self) -> String {
        let high = Summary::of(&self.times[Fidelity::High.index() as usize]).median;
        let mut lines = vec![
            "fidelity  frames  mean ms  median ms  p95 ms  p99 ms  median vs High".to_string(),
        ];
        for (fidelity, times) in Fidelity::ALL.into_iter().zip(&self.times) {
            let s = Summary::of(times);
            lines.push(format!(
                "{:<8}  {:>6}  {:>7.2}  {:>9.2}  {:>6.2}  {:>6.2}  {:>13.2}",
                fidelity.name(),
                times.len(),
                s.mean,
                s.median,
                s.p95,
                s.p99,
                if high > 0. { s.median / high } else { 0. },
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
    fn steps_take_turns_then_each_replays_to_the_saved_frame() {
        assert_eq!(Bench::phase(0), Phase::Timing(Fidelity::Low, 0));
        assert_eq!(Bench::phase(BLOCK), Phase::Timing(Fidelity::Medium, BLOCK));
        assert_eq!(
            Bench::phase(BLOCK * 3),
            Phase::Timing(Fidelity::Ultra, BLOCK * 3)
        );
        assert_eq!(
            Bench::phase(BLOCK * 4),
            Phase::Timing(Fidelity::Low, BLOCK * 4)
        );
        assert_eq!(
            Bench::phase(SCRIPT + 1),
            Phase::Timing(Bench::fidelity(SCRIPT + 1), 1)
        );
        assert_eq!(Bench::phase(TIMED), Phase::Picture(Fidelity::Low, 0));
        let ultra_shot = TIMED + 3 * (SHOT + 1) + SHOT;
        assert_eq!(
            Bench::phase(ultra_shot),
            Phase::Picture(Fidelity::Ultra, SHOT)
        );
        assert!(Bench::shot(ultra_shot) && !Bench::shot(ultra_shot - 1));
        assert_eq!(Bench::phase(ultra_shot + 1), Phase::Finished);
        let shots = (0..ultra_shot + 1).filter(|f| Bench::shot(*f)).count();
        assert_eq!(shots, 4, "one picture per step");
    }

    #[test]
    fn warm_up_and_each_blocks_first_frames_are_left_out() {
        let mut bench = Bench::default();
        let mut frame = 0;
        while Bench::phase(frame) != Phase::Finished {
            bench.mark(frame, frame as f64 * 2.);
            frame += 1;
        }
        let timed: usize = bench.times.iter().map(Vec::len).sum();
        let expected = (0..TIMED)
            .filter(|f| f % SCRIPT >= WARMUP && f % BLOCK >= SETTLE)
            .count();
        assert_eq!(timed, expected);
        for times in &bench.times {
            assert!(times.len() > 300, "every step is timed throughout");
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
