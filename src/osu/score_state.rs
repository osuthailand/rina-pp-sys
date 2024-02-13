/// Aggregation for a score's current state.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct OsuScoreState {
    /// Maximum combo that the score has had so far.
    /// **Not** the maximum possible combo of the map so far.
    pub max_combo: u32,
    /// Amount of current 300s.
    pub n300: u32,
    /// Amount of current 100s.
    pub n100: u32,
    /// Amount of current 50s.
    pub n50: u32,
    /// Amount of current misses.
    pub n_misses: u32,
}

impl OsuScoreState {
    /// Create a new empty score state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the total amount of hits by adding everything up.
    pub const fn total_hits(&self) -> u32 {
        self.n300 + self.n100 + self.n50 + self.n_misses
    }

    /// Calculate the accuracy between `0.0` and `1.0` for this state.
    pub fn accuracy(&self) -> f64 {
        let total_hits = self.total_hits();

        if total_hits == 0 {
            return 0.0;
        }

        let numerator = 6 * self.n300 + 2 * self.n100 + self.n50;
        let denominator = 6 * total_hits;

        f64::from(numerator) / f64::from(denominator)
    }
}