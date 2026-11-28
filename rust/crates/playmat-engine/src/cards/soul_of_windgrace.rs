//! Soul of Windgrace: "Whenever Soul of Windgrace enters or attacks, you may put a land card from a graveyard onto
//! the battlefield tapped under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SoulOfWindgrace.SoulOfWindgraceEffect", apply)];

/// `choose(TargetCardInGraveyard(0, 1, land))` over every graveyard (players from you in turn order), asked even
/// with no land card there; the card chosen enters tapped under your control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        cards.extend(game.player(p).graveyard.iter().copied().filter(|c| game.has_type(*c, CardType::Land)));
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &cards, &cards, 0, 1, &rule)?;
    if !put_cards(game, d, you, &chosen, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    Some(())
}
