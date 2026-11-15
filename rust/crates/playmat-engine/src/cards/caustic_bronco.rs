//! Caustic Bronco: "Whenever Caustic Bronco attacks, reveal the top card of your library and put it into your hand.
//! You lose life equal to that card's mana value if Caustic Bronco isn't saddled. Otherwise, each opponent loses
//! that much life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CausticBronco.CausticBroncoEffect", apply)];

/// The reveal and the card to hand, then `SaddledCondition` as the life loss is chosen
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let &card = game.player(you).library.first()?;
    let zcc = game.obj(cx.source_id()).zcc;
    let cards = vec![(card, Zone::Library)];
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    game.move_to(card, Zone::Hand);
    let mv = game.obj(card).def.mana_value() as i32;
    if !game.condition_holds(Some(&Condition::SourceSaddled), you, cx.source_id()) {
        lose_life(game, you, mv);
        return Some(());
    }
    for i in 1..game.seats() {
        lose_life(game, (you + i) % game.seats(), mv);
    }
    Some(())
}
