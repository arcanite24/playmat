//! Dream Harvest: "Each opponent exiles cards from the top of their library until they have exiled cards with total
//! mana value 5 or greater this way. Until end of turn, you may cast cards exiled this way without paying their mana
//! costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DreamHarvest.DreamHarvestEffect", apply)];

/// Each opponent moves their top cards to exile one at a time until the ones moved total mana value 5 or more; then
/// `PlayFromNotOwnHandZoneTargetEffect(EXILED, YOU, EndOfTurn, withoutMana, onlyCast)` on all of them
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    let mut exiled: Vec<ObjId> = Vec::new();
    for p in (1..n).map(|i| (you + i) % n) {
        if game.player(p).lost {
            continue;
        }
        let library = game.player(p).library.clone();
        let mut total = 0;
        for card in library {
            game.move_to(card, Zone::Exile);
            exiled.push(card);
            total += game.obj(card).def.mana_value();
            if total >= 5 {
                break;
            }
        }
    }
    let turn = game.turn;
    for c in exiled {
        if game.obj(c).zone == Zone::Exile {
            let o = game.obj_mut(c);
            o.may_play = Some((you, turn));
            o.play_free = true;
            o.cast_only = true;
        }
    }
    Some(())
}
