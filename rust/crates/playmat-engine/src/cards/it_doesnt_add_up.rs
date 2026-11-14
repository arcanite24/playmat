//! It Doesn't Add Up: "Return target creature card from your graveyard to the battlefield. Suspect it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ItDoesntAddUp.ItDoesntAddUpEffect", apply)];

/// the card onto the battlefield under the controller's control, then the permanent it became is suspected
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    put_onto_battlefield(game, d, card, cx.controller());
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        game.obj_mut(card).set_suspected(true);
    }
    Some(())
}
