//! Questions the engine asks a seat, mirroring the client queries `HumanPlayer` fires (docs/rust/HumanDialogs.md), and
//! the answers a client could send.

use crate::state::{Game, ObjId, Seat};

/// A question for one seat. Each names the Java query it mirrors (`PlayerQueryEvent.QueryType`).
#[derive(Clone, Debug, PartialEq)]
pub enum Query {
    /// SELECT "Play spells and abilities": an object to play or activate, or a pass
    Priority { playable: Vec<ObjId> },
    /// PLAY_MANA "Pay …": a permanent with a mana ability, or cancel
    PlayMana { cost: String, sources: Vec<ObjId> },
    /// PICK_TARGET: one of the legal targets; `required` targets can't be cancelled
    Target { players: Vec<Seat>, objs: Vec<ObjId>, required: bool },
    /// SELECT "Select attackers": a creature to toggle, or done
    Attackers { creatures: Vec<ObjId> },
    /// SELECT "Select blockers": a creature to block with, then the attacker it blocks; or done
    Blockers { creatures: Vec<ObjId> },
    /// TARGET "Select an attacker to block"
    BlockTarget { attackers: Vec<ObjId> },
    /// PICK_TARGET for a discard to hand size, or another choice of cards
    Cards { objs: Vec<ObjId>, required: bool },
    /// ASK: yes or no
    YesNo { message: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    Bool(bool),
    Int(i32),
    Text(String),
    ManaType(String),
    Player(Seat),
    Obj(ObjId),
    /// no answer: the decider gave up (a replay that left Java's game); the engine stops
    Stop,
}

impl Query {
    /// Whether a client could send this answer to this question. Java records every answer, including ones the
    /// dialog then ignores (an unplayable card, a tapped land), and asks again; the engine does the same, so only
    /// answers no client could send at all are refused.
    pub fn accepts(&self, answer: &Answer) -> bool {
        !matches!((self, answer), (Query::YesNo { .. }, Answer::Obj(_) | Answer::Player(_)))
    }
}

/// Answers the engine's questions. The engine calls `traced` where Java traces (each priority, each resolution).
pub trait Decider {
    fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer;

    fn traced(&mut self, _game: &Game) {}

    /// the engine checks this between steps and stops the game when it's true
    fn stopped(&self) -> bool {
        false
    }
}

/// Passes every question: never plays anything, never attacks or blocks.
pub struct Passer;

impl Decider for Passer {
    fn answer(&mut self, _game: &Game, _seat: Seat, query: &Query) -> Answer {
        match query {
            Query::Target { objs, players, required: true } => players
                .first()
                .map(|s| Answer::Player(*s))
                .or(objs.first().map(|o| Answer::Obj(*o)))
                .unwrap_or(Answer::Stop),
            Query::Cards { objs, required: true } => objs.first().map(|o| Answer::Obj(*o)).unwrap_or(Answer::Stop),
            _ => Answer::Bool(false),
        }
    }
}
