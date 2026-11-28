//! Augmenter Pugilist // Echoing Equation: "Choose target creature you control. Each other creature you control
//! becomes a copy of it until end of turn, except those creatures aren't legendary if the chosen creature is
//! legendary."

use super::*;
use std::sync::Arc;

pub const HANDLERS: &[Handler] = &[Handler::effect("AugmenterPugilist.EchoingEquationEffect", apply)];

/// The target still legal: each other creature you control, in battlefield order, `copyPermanent(Duration.EndOfTurn,
/// …)` of it with the applier taking legendary away
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.obj(0) else { return Some(()) };
    let mut copy = (*game.obj(of).copiable()).clone();
    copy.supertypes.retain(|s| s != "LEGENDARY");
    let copy = Arc::new(copy);
    let others: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != of && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    for o in others {
        become_copy_until_end_of_turn(game, o, copy.clone());
    }
    Some(())
}
