//! Chandra's Defeat: "Chandra's Defeat deals 5 damage to target red creature or red planeswalker. If it was a
//! Chandra planeswalker, you may discard a card. If you do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChandrasDefeat.ChandrasDefeatEffect", apply)];

/// the Chandra check after the damage, on the permanent as it then is; the draw comes with the discard asked for
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    deal_damage(game, cx.source_id(), Target::Obj(o), 5, false);
    let chandra = game.has_type(o, CardType::Planeswalker) && game.has_subtype(o, "Chandra");
    let you = cx.controller();
    if chandra && choose_use(game, d, you, "Discard a card and draw a card?")? {
        discard(game, d, you, 1);
        if d.stopped() {
            return None;
        }
        draw(game, you, 1);
    }
    Some(())
}
