//! Illuna, Apex of Wishes: "Whenever this creature mutates, exile cards from the top of your library until you exile
//! a nonland permanent card. Put that card onto the battlefield or into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IllunaApexOfWishes.IllunaApexOfWishesEffect", apply)];

/// The cards down to the first nonland permanent card, exiled together (`moveCards`); then `chooseUse` ("Hand" is
/// yes) and that card moved there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    let mut hit = None;
    for c in game.player(you).library.clone() {
        cards.push(c);
        let def = &game.obj(c).def;
        if !def.is(CardType::Land) && def.is_permanent_card() {
            hit = Some(c);
            break;
        }
    }
    if !cards.is_empty() {
        put_cards(game, d, you, &cards, crate::card::PutCards::Exiled);
    }
    if d.stopped() {
        return None;
    }
    let Some(card) = hit else { return Some(()) };
    let message = format!("Put {} into your hand or onto the battlefield?", game.obj(card).def.name);
    if choose_use(game, d, you, &message)? {
        game.move_to(card, Zone::Hand);
    } else {
        put_onto_battlefield(game, d, card, you);
    }
    Some(())
}
