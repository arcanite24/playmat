//! Etali, Primal Conqueror: "When Etali enters, each player exiles cards from the top of their library until they
//! exile a nonland card. You may cast any number of spells from among the nonland cards exiled this way without
//! paying their mana costs." Etali, Primal Sickness: "Whenever Etali deals combat damage to a player, they get that
//! many poison counters."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("EtaliPrimalConqueror.EtaliPrimalConquerorEffect", apply),
    Handler::effect("EtaliPrimalConqueror.EtaliPrimalSicknessEffect", sickness),
];

/// Each player from you in turn order exiles their library's cards one by one until a nonland one; then the nonland
/// cards still in exile go to `castMultipleWithAttributeForFree`, as Etali, Primal Storm's do
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let library: Vec<ObjId> = game.player(p).library.clone();
        for c in library {
            game.move_to(c, Zone::Exile);
            if !game.obj(c).is(CardType::Land) {
                cards.push(c);
                break;
            }
        }
    }
    let cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    super::etali_primal_storm::cast_multiple_for_free(game, d, cx, you, cards)
}

/// The damaged player (the target pointer) gets the damage dealt (`getValue("damage")`) in poison counters
fn sickness(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(p) = cx.player(0) {
        add_poison(game, p, cx.item.x);
    }
    Some(())
}
