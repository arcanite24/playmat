//! The game's random numbers: `java.util.Random`'s linear congruential generator, as Java's `GameRandom` uses it, so a
//! seeded game shuffles and flips the same way in both engines.

const MULTIPLIER: i64 = 0x5DEECE66D;
const ADDEND: i64 = 0xB;
const MASK: i64 = (1 << 48) - 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaRandom {
    state: i64,
}

impl JavaRandom {
    pub fn new(seed: i64) -> Self {
        JavaRandom { state: (seed ^ MULTIPLIER) & MASK }
    }

    pub fn state(&self) -> i64 {
        self.state
    }

    /// `GameRandom.setState`: continue from a recorded position
    pub fn set_state(&mut self, state: i64) {
        self.state = state & MASK;
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.state = (self.state.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        (self.state >> (48 - bits)) as i32
    }

    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// `Random.nextInt(bound)`
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let mut r = self.next(31);
        let m = bound - 1;
        if bound & m == 0 {
            return ((bound as i64 * r as i64) >> 31) as i32;
        }
        let mut u = r;
        loop {
            r = u % bound;
            if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                return r;
            }
            u = self.next(31);
        }
    }

    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    /// `Library.shuffle`: from the last place down, swap with a random earlier one (Fisher–Yates as Java writes it)
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        let mut n = items.len();
        while n > 1 {
            n -= 1;
            let r = self.next_int_bound(n as i32 + 1) as usize;
            items.swap(n, r);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_java_util_random() {
        // new java.util.Random(42): nextInt() x3, nextInt(10) x3, nextLong()
        let mut r = JavaRandom::new(42);
        assert_eq!(r.next_int(), -1170105035);
        assert_eq!(r.next_int(), 234785527);
        assert_eq!(r.next_int(), -1360544799);
        let mut r = JavaRandom::new(42);
        assert_eq!(r.next_int_bound(10), 0);
        assert_eq!(r.next_int_bound(10), 3);
        assert_eq!(r.next_int_bound(10), 8);
        assert_eq!(r.next_int_bound(7), 3);
        assert_eq!(r.next_int_bound(1_000_000_007), 662969970);
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_long(), -4962768465676381896);
        let mut r = JavaRandom::new(5);
        assert!(r.next_boolean());
        assert_eq!(r.next_int_bound(16), 2);
    }
}
