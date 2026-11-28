//! Ugin, the Spirit Dragon: "-X: Exile each permanent with mana value X or less that's one or more colors." and
//! "-10: You gain 7 life, draw seven cards, then put up to seven permanent cards from your hand onto the
//! battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("UginTheSpiritDragon.UginTheSpiritDragonEffect2", exile_colored),
    Handler::effect("UginTheSpiritDragon.UginTheSpiritDragonEffect3", ultimate),
];

/// Every colored permanent with mana value X or less, exiled together (`moveCards`)
fn exile_colored(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    let doomed: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).def.mana_value() as i32 <= x && !game.colors(*o).is_empty())
        .collect();
    for o in doomed {
        game.move_to(o, Zone::Exile);
    }
    Some(())
}

/// Gain 7, draw 7, then up to seven permanent cards from the hand (`TargetCardInHand(0, 7)`) onto the battlefield
fn ultimate(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    gain_life(game, you, 7);
    draw(game, you, 7);
    let hand = game.player(you).hand.clone();
    let permanents: Vec<ObjId> = hand.iter().copied().filter(|c| game.obj(*c).def.is_permanent_card()).collect();
    if permanents.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &hand, &permanents, 0, 7, &rule)?;
    if !chosen.is_empty() {
        put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield);
    }
    Some(())
}
