//! Lluwen, Imperfect Naturalist: "When Lluwen enters, mill four cards, then you may put a creature or land card from
//! among the milled cards on top of your library. {2}{B/G}{B/G}{B/G}, {T}, Discard a land card: Create a 1/1 black
//! and green Worm creature token for each land card in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LluwenImperfectNaturalist.LluwenImperfectNaturalistEffect", apply)];

/// `millCards(4)` (the top four, wherever they went), then `player.choose(cards, TargetCard(0, 1, Zone.ALL,
/// creature or land))` over them, the chosen one put on top of your library
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(4).copied().collect();
    cx.run_on(game, d, &[EffectDef::MillCardsTargetEffect { amount: Amount::Fixed(4) }], vec![Target::Player(you)]);
    if d.stopped() {
        return None;
    }
    let milled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone != Zone::Library).collect();
    if milled.is_empty() {
        return Some(());
    }
    let possible: Vec<ObjId> = milled
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Creature) || game.obj(*c).is(CardType::Land))
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &milled, &possible, 0, 1, &rule)?;
    if let Some(c) = chosen.first() {
        to_library(game, *c, true);
    }
    Some(())
}
