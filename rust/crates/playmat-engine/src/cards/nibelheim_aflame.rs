//! Nibelheim Aflame: "Choose target creature you control. It deals damage equal to its power to each other
//! creature. If this spell was cast from a graveyard, discard your hand and draw four cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NibelheimAflame.NibelheimAflameEffect", apply)];

/// The damage part: each other creature, in battlefield order (the rest is the card's next effect)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(o).0;
    if power < 1 {
        return Some(());
    }
    let others: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|&c| c != o && game.has_type(c, CardType::Creature)).collect();
    for c in others {
        deal_damage(game, o, Target::Obj(c), power, false);
    }
    Some(())
}
