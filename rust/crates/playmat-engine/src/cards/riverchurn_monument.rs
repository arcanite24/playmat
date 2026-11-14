//! Riverchurn Monument's exhaust: "Any number of target players each mill cards equal to the number of cards in
//! their graveyard."

use super::*;
use crate::card::{Amount, EffectDef};
use crate::state::Target;

pub const HANDLERS: &[Handler] = &[Handler::effect("RiverchurnMonument.RiverchurnMonumentEffect", apply)];

/// each target player in turn mills as many cards as their graveyard has then (`millCards(graveyard.size())`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let players: Vec<Seat> = cx
        .legal
        .first()
        .map(|ts| ts.iter().filter_map(|t| if let Target::Player(p) = t { Some(*p) } else { None }).collect())
        .unwrap_or_default();
    for p in players {
        let n = game.player(p).graveyard.len() as i32;
        let mill = EffectDef::MillCardsTargetEffect { amount: Amount::Fixed(n) };
        cx.run_on(game, d, &[mill], vec![Target::Player(p)]);
    }
    Some(())
}
