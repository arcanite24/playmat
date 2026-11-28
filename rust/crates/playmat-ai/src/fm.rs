//! The float functions the AI's game path calls (`exp`, `ln`, `tanh`), routed so that the `strict` build computes
//! them the same way on every machine (Mana@Home, docs/swarm/Design.md, "Versions, staleness and determinism").
//!
//! `f64::exp` and friends call the platform's maths library (Apple's, glibc's, Microsoft's), which can differ in the
//! last bit; one bit is enough for a search to pick another move. The strict build uses the `libm` crate (musl's
//! code, in Rust) instead. Basic arithmetic and `sqrt` are IEEE-exact everywhere and Rust never fuses `a * b + c`
//! on its own, so they need nothing.

pub trait Fm {
    fn sexp(self) -> Self;
    fn sln(self) -> Self;
    fn stanh(self) -> Self;
}

#[cfg(not(feature = "strict"))]
impl Fm for f64 {
    #[inline(always)]
    fn sexp(self) -> f64 {
        self.exp()
    }
    #[inline(always)]
    fn sln(self) -> f64 {
        self.ln()
    }
    #[inline(always)]
    fn stanh(self) -> f64 {
        self.tanh()
    }
}

#[cfg(not(feature = "strict"))]
impl Fm for f32 {
    #[inline(always)]
    fn sexp(self) -> f32 {
        self.exp()
    }
    #[inline(always)]
    fn sln(self) -> f32 {
        self.ln()
    }
    #[inline(always)]
    fn stanh(self) -> f32 {
        self.tanh()
    }
}

#[cfg(feature = "strict")]
impl Fm for f64 {
    #[inline(always)]
    fn sexp(self) -> f64 {
        libm::exp(self)
    }
    #[inline(always)]
    fn sln(self) -> f64 {
        libm::log(self)
    }
    #[inline(always)]
    fn stanh(self) -> f64 {
        libm::tanh(self)
    }
}

#[cfg(feature = "strict")]
impl Fm for f32 {
    #[inline(always)]
    fn sexp(self) -> f32 {
        libm::expf(self)
    }
    #[inline(always)]
    fn sln(self) -> f32 {
        libm::logf(self)
    }
    #[inline(always)]
    fn stanh(self) -> f32 {
        libm::tanhf(self)
    }
}

#[cfg(test)]
mod tests {
    use super::Fm;

    #[test]
    fn the_functions_are_the_usual_ones() {
        assert!((1.0f64.sexp() - std::f64::consts::E).abs() < 1e-15);
        assert!((std::f32::consts::E.sln() - 1.0).abs() < 1e-6);
        assert!((0.5f64.stanh() - 0.5f64.tanh()).abs() < 1e-15);
    }
}
