//! The game host (docs/rust/GameHost.md, step D): plays a duel on the Rust engine for the web client, a human
//! against the Rust AI, speaking the client's game protocol (Java's callbacks and answers).

pub mod game;
pub mod ids;
pub mod prompts;
pub mod record;
pub mod skips;
pub mod timer;

pub use game::{Call, Ended, Event, HostedGame, SeatStart, Start};
pub use timer::Timer;
