//! Fear of Immobility: "When this creature enters, tap up to one target creature. If an opponent controls that
//! creature, put a stun counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FearOfImmobility.FearOfImmobilityEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone == Zone::Battlefield && game.obj(o).controller != you {
        put_counters(game, o, "stun", 1, you);
    }
    Some(())
}
