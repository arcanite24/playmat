//! Check for Traps: "Target opponent reveals their hand. You choose a nonland card from it. Exile that card. If an
//! instant card or a card with flash is exiled this way, they lose 1 life. Otherwise, you lose 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CheckForTraps.CheckForTrapsEffect", apply)];

/// The hand revealed; `choose(TargetCard(HAND, nonland))`, asked unless the hand is empty; the card exiled; an
/// instant or a card with flash costs them 1 life, else you lose 1
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let hand = game.player(p).hand.clone();
    let mut theirs = false;
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
        let possible: Vec<ObjId> = hand.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
        let rule = item_rule(game, cx.item);
        if let Some(&c) = choose_cards(game, d, you, &hand, &possible, 1, 1, &rule)?.first() {
            game.move_to(c, Zone::Exile);
            let def = &game.obj(c).def;
            theirs = def.is(CardType::Instant) || def.abilities.contains(&AbilityDef::FlashAbility);
        }
    }
    let src = Some(cx.source_id());
    lose_life_by(game, if theirs { p } else { you }, 1, src, false);
    Some(())
}
