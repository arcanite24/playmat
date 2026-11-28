//! Flesh // Blood: "Exile target creature card from a graveyard. Put X +1/+1 counters on target creature, where X is
//! the power of the card you exiled. // Target creature you control deals damage equal to its power to any target."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FleshBlood.FleshEffect", apply)];

/// `game.getCard(source.getFirstTarget())`: nothing when the card is gone. Its power (as a card), the exile, then
/// the counters on the second target if that power is above 0 and it's still a legal permanent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = cx.obj(0)?;
    let power = game.power_toughness(card).0;
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Exiled) {
        return None;
    }
    if power > 0 {
        if let Some(c) = cx.obj(1).filter(|c| game.obj(*c).zone == Zone::Battlefield) {
            add_counters_by(game, c, "+1/+1", power, Some(you));
        }
    }
    Some(())
}
