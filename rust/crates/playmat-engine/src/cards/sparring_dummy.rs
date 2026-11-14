//! Sparring Dummy: "{T}: Mill a card. You may put a land card milled this way into your hand. You gain 2 life if a
//! Lesson card is milled this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SparringDummy.SparringDummyEffect", apply)];

/// `choose(cards, TargetCard(0, 1, land))` among the milled card, then the Lesson check on what was milled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(1).copied().collect();
    cx.run_on(game, d, &[EffectDef::MillCardsTargetEffect { amount: Amount::Fixed(1) }], vec![Target::Player(you)]);
    if d.stopped() {
        return None;
    }
    let milled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    let lands: Vec<ObjId> = milled.iter().copied().filter(|c| game.obj(*c).is(CardType::Land)).collect();
    let rule = item_rule(game, cx.item);
    if !milled.is_empty() {
        if let Some(l) = choose_cards(game, d, you, &milled, &lands, 0, 1, &rule)?.first() {
            game.move_to(*l, Zone::Hand);
        }
    }
    if milled.iter().any(|c| game.has_subtype(*c, "Lesson")) {
        gain_life(game, you, 2);
    }
    Some(())
}
