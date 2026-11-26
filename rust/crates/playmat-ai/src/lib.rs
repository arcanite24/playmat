//! Playmat's Rust AI (docs/rust/Ai.md): searches with whole-turn playouts on copies of the Rust engine's games.

/// Whether this build is strict (`fm.rs`): one determinism class on every machine. Read it from here, so a crate
/// that enables `playmat-ai/strict` without its own `strict` feature still says what it computes
pub const STRICT: bool = cfg!(feature = "strict");

pub mod answerlog;
pub mod combat;
pub mod decision;
pub mod duel;
pub mod entrants;
pub mod eval;
pub mod fitted;
pub mod fm;
pub mod mirror;
pub mod mulligan;
pub mod net;
pub mod obs;
pub mod play;
pub mod playbook;
pub mod policy;
pub mod search;
pub mod selfplay;
pub mod tree;
