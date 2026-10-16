//! Random numbers for the choices the game makes at random
//!
//! The game uses `Random`. A seeded generator makes the choices repeatable, e.g. for tests or so
//! that a placement looks the same every time the map is loaded.

/// A SplitMix64 generator
#[derive(Clone, Debug)]
pub struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// A number in `0..range`, 0 if `range` is 0 (`Random`)
    pub fn next(&mut self, range: u32) -> u32 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        (z % u64::from(range.max(1))) as u32
    }

    pub fn pick<T: Copy>(&mut self, choices: &[T]) -> T {
        choices[self.next(choices.len() as u32) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_choices_are_repeatable() {
        let mut a = Random::new(5);
        let mut b = Random::new(5);
        let first: Vec<_> = (0..10).map(|_| a.next(100)).collect();
        let second: Vec<_> = (0..10).map(|_| b.next(100)).collect();
        assert_eq!(first, second);
        assert!(first.iter().all(|&n| n < 100));
        assert_eq!(a.next(0), 0);
    }
}
