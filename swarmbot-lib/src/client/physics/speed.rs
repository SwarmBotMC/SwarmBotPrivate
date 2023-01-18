#[derive(Debug, PartialEq, Copy, Clone, Hash, Ord, PartialOrd, Eq)]
pub struct MovementSpeed {
    multiplier: f64,
}

impl Default for MovementSpeed {
    fn default() -> Self {
        Self::STOP
    }
}

impl MovementSpeed {
    const fn new(multiplier: f64) -> Self {
        Self { multiplier }
    }

    pub const SPRINT: Self = Self::new(1.3);
    pub const WALK: Self = Self::new(1.0);
    pub const SNEAK: Self = Self::new(0.3);
    pub const STOP: Self = Self::new(0.);

    pub fn multiplier(&self) -> f64 {
        self.multiplier * 0.98 // TODO: different at 45 degree angle
    }

    pub const ALL: [Self; 4] = [Self::SPRINT, Self::WALK, Self::SNEAK, Self::STOP];
}
