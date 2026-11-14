//! Perilous Voyage: "Return target nonland permanent you don't control to its owner's hand. If its mana value was 2 or
//! less, scry 2."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PerilousVoyage.PerilousVoyageEffect", apply)];

/// The mana value is read before the permanent leaves; then it goes to its owner's hand and maybe you scry 2
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    let small = game.obj(t).def.mana_value() <= 2;
    put_cards(game, d, you, &[t], crate::card::PutCards::Hand);
    if d.stopped() {
        return None;
    }
    if small {
        let rule = item_rule(game, cx.item);
        if !scry(game, d, you, 2, &rule) {
            return None;
        }
    }
    Some(())
}
