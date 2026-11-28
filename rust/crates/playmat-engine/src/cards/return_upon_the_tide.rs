//! Return Upon the Tide: "Return target creature card from your graveyard to the battlefield. If it's an Elf, create
//! two 1/1 green Elf Warrior creature tokens."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReturnUponTheTide.ReturnUponTheTideEffect", apply)
    .effects(&["new CreateTokenEffect(new ElfWarriorToken(), 2)"])];

/// The target card, if still legal, onto the battlefield under you; the permanent it became, an Elf, makes the
/// two tokens
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    if !put_cards(game, d, cx.controller(), &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield && game.has_subtype(card, "Elf") {
        cx.run_on(game, d, &cx.effects[..1], Vec::new());
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
