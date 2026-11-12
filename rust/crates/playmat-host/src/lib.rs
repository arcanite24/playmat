//! The game host (docs/rust/GameHost.md, step D): plays a duel on the Rust engine for the web client, a human
//! against the Rust AI, speaking the client's game protocol (Java's callbacks and answers).

pub mod game;
pub mod ids;
pub mod prompts;
pub mod record;
pub mod replay;
pub mod skips;
pub mod tally;
pub mod timer;

pub use game::{Call, Calls, Ended, Event, HostedGame, Prelude, SeatStart, Start, TableRules};
pub use timer::Timer;
