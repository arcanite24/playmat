//! Birgi, God of Storytelling // Harnfel, Horn of Bounty: "Creatures you control can boast twice during each of your
//! turns rather than once."

use super::*;
use crate::card::Condition;

pub const HANDLERS: &[Handler] = &[Handler::rule(
    "BirgiGodOfStorytelling.BirgiGodOfStorytellingEffect",
    Rule::MaxActivations(twice),
)];

/// `MAX_ACTIVATIONS` of a `BoastAbility` activated by Birgi's controller: 2 (Java doesn't check whose turn it is)
fn twice(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, index: usize) -> Option<u32> {
    let boast = matches!(
        game.obj(id).def.abilities.get(index),
        Some(AbilityDef::LimitedTimesPerTurnActivatedAbility {
            limit: 1,
            condition: Some(Condition::SourceAttackedThisTurn),
            ..
        })
    );
    (boast && seat == cx.controller).then_some(2)
}
