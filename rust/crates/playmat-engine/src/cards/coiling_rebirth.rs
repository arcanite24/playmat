//! Coiling Rebirth: "Gift a card. Return target creature card from your graveyard to the battlefield. Then if the
//! gift was promised and that creature isn't legendary, create a token that's a copy of that creature, except it's
//! 1/1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CoilingRebirth.CoilingRebirthEffect", apply)];

/// The target still in the graveyard onto the battlefield; with the gift promised and it there and not legendary,
/// a 1/1 token copy of it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if !cx.item.gift_promised || game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    if game.obj(card).def.supertypes.iter().any(|s| s == "LEGENDARY") {
        return Some(());
    }
    token_copy_modified(game, d, card, you, 1, &|def: &mut CardDef| {
        def.remove_pt_cda();
        def.power = 1;
        def.toughness = 1;
    });
    if d.stopped() {
        return None;
    }
    Some(())
}
