//! The Career's chance, seeded as Java seeds it (`CareerDice`, docs/rust/Payouts.md "Dice"): a roll is
//! `java.util.Random` seeded with the first eight bytes of SHA-256 over career.db's secret, a newline and what the
//! roll is for. Java's `Random` is ported here draw for draw, so both sides pick the same quests and capsules.

use sha2::{Digest, Sha256};

/// `java.util.Random`: the 48-bit linear congruential generator, as the JDK documents it
pub struct JavaRandom {
    seed: i64,
}

const MULTIPLIER: i64 = 0x5DEECE66D;
const MASK: i64 = (1 << 48) - 1;

impl JavaRandom {
    /// `new Random(seed)`
    pub fn new(seed: i64) -> JavaRandom {
        JavaRandom { seed: (seed ^ MULTIPLIER) & MASK }
    }

    /// `next(bits)`
    fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(0xB)) & MASK;
        ((self.seed as u64) >> (48 - bits)) as i32
    }

    /// `nextInt(bound)`, bound > 0
    pub fn next_int(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let mut r = self.next(31);
        let m = bound - 1;
        if bound & m == 0 {
            return ((i64::from(bound) * i64::from(r)) >> 31) as i32;
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

    /// `nextLong()`
    pub fn next_long(&mut self) -> i64 {
        (i64::from(self.next(32)) << 32).wrapping_add(i64::from(self.next(32)))
    }
}

/// `CareerDice.seed`
pub fn seed(secret: &str, purpose: &str) -> i64 {
    let digest = Sha256::digest(format!("{secret}\n{purpose}").as_bytes());
    i64::from_be_bytes(digest[..8].try_into().expect("eight bytes"))
}

/// `CareerDice.roll`
pub fn roll(secret: &str, purpose: &str) -> JavaRandom {
    JavaRandom::new(seed(secret, purpose))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_as_java_does() {
        // Java 21: new Random(42): nextInt(10) five times, then nextInt(16); new Random(-7): nextInt, nextLong
        let mut random = JavaRandom::new(42);
        let draws: Vec<i32> = (0..5).map(|_| random.next_int(10)).collect();
        assert_eq!(draws, [0, 3, 8, 4, 0]);
        assert_eq!(random.next_int(16), 15);
        let mut random = JavaRandom::new(-7);
        assert_eq!(random.next_int(1_000_000_007), 577_934_662);
        assert_eq!(random.next_long(), -1_817_049_056_290_694_612);
    }

    #[test]
    fn seeds_as_java_does() {
        // CareerDice.seed("s3cret", "quest/ana/2026-11-11/0"), and its first nextInt(37)
        assert_eq!(seed("s3cret", "quest/ana/2026-11-11/0"), -7_933_311_343_224_870_592);
        assert_eq!(roll("s3cret", "quest/ana/2026-11-11/0").next_int(37), 34);
    }
}
