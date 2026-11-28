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
    /// MULTI_AMOUNT: one amount per item (combat damage among blockers), each within its range, summing to between
    /// `min` and `max`; answered as a string of space-separated numbers. For combat damage, `source` is the creature
    /// dealing it and `items` the creatures it's divided among, one per amount; otherwise both are empty.
    MultiAmount {
        defaults: Vec<i32>,
        ranges: Vec<(i32, i32)>,
        min: i32,
        max: i32,
        source: Option<ObjId>,
        items: Vec<ObjId>,
    },
    /// CHOOSE_ABILITY: which of a permanent's abilities (by index) to activate
    ChooseAbility { source: ObjId, abilities: Vec<usize> },
    /// PICK_ABILITY: which of the player's `count` waiting triggers goes on the stack next; `triggers` names each, in
    /// the same order: its source and its rule text
    PickTrigger { count: usize, triggers: Vec<(ObjId, String)> },
    /// ASK: yes or no
    YesNo { message: String },
    /// CHOOSE_MODE: one of a modal spell's modes (by index)
    ChooseMode { modes: Vec<usize> },
    /// AMOUNT: a number between `min` and `max` (announcing X)
    Amount { min: i32, max: i32 },
    /// CHOOSE_CHOICE: one of some texts (a color of mana), answered as a string
    Choice { message: String, choices: Vec<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    Bool(bool),
    Int(i32),
    Text(String),
    ManaType(String),
    Player(Seat),
    Obj(ObjId),
    /// an object's ability, by its index in the card's abilities (`B3a2`)
    Ability(ObjId, usize),
    /// a waiting trigger of the asked player, by its place in their list (`T0.1`)
    Trigger(usize),
    /// CHOOSE_MODE's Done (`MD`)
    ModeDone,
    /// CHOOSE_MODE's Cancel (`MC`)
    ModeCancel,
    /// a modal spell's mode, by index (`S0sm2`)
    Mode(usize),
    /// an id the decision log couldn't name (a dialog's Done or Cancel option)
    Unknown,
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

    /// the engine checks this as each step ends (after its end-of-step work, as Java's `Phase.checkStopOnStepOption`)
    /// and stops the game there when it's true: exported tests' `stopOnTurn` and `stopAtStep`
    fn stop_after(&mut self, _game: &Game, _step: crate::state::Step) -> bool {
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
