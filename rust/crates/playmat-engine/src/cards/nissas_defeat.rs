//! Nissa's Defeat: "Destroy target Forest, green enchantment, or green planeswalker. If that permanent was a Nissa
//! planeswalker, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NissasDefeat.NissasDefeatEffect", apply)];

/// The destroy, then a draw when it was a Nissa planeswalker (as it last was on the battlefield)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let nissa = game.has_type(o, CardType::Planeswalker) && game.has_subtype(o, "Nissa");
    destroy(game, o);
    if nissa {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
