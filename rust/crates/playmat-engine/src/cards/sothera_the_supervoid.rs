//! Sothera, the Supervoid: "Whenever a creature you control dies, each opponent chooses a creature they control and
//! exiles it. At the beginning of your end step, if a player controls no creatures, sacrifice Sothera, then put a
//! creature card exiled with it onto the battlefield under your control with two additional +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("SotheraTheSupervoid.SotheraTheSupervoidCondition", a_player_without_creatures),
    Handler::effect("SotheraTheSupervoid.SotheraTheSupervoidExileEffect", exile),
    Handler::effect("SotheraTheSupervoid.SotheraTheSupervoidReturnEffect", put_back),
];

/// a player in range (still in the game) controls no creatures
fn a_player_without_creatures(game: &Game, _you: Seat, _source: ObjId) -> bool {
    (0..game.seats()).any(|p| !game.player(p).lost && controlled_creatures(game, p).is_empty())
}

/// each opponent with a creature chooses one they control (not targeted) and exiles it into this source's exile zone
fn exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    for p in opponents {
        let can = controlled_creatures(game, p);
        if can.is_empty() {
            continue;
        }
        let Some(o) = choose_cards(game, d, p, &can, &can, 1, 1, "to exile")?.first().copied() else { continue };
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        game.move_to(o, Zone::Exile);
        if game.obj(o).zone == Zone::Exile {
            game.obj_mut(o).exiled_with = Some(cx.source);
        }
    }
    Some(())
}

/// a creature card exiled with this (the only one, or one chosen), put onto the battlefield under your control with
/// two +1/+1 counters (`setEnterWithCounters`)
fn put_back(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| game.obj(*c).exiled_with == Some(cx.source) && game.obj(*c).def.types.contains(&CardType::Creature))
        .collect();
    let card = match creatures.len() {
        0 => return Some(()),
        1 => creatures[0],
        _ => match choose_cards(game, d, you, &creatures, &creatures, 1, 1, "put onto the battlefield")?.first() {
            Some(c) => *c,
            None => return Some(()),
        },
    };
    enter_battlefield(game, d, card, you, false);
    if game.obj(card).zone == Zone::Battlefield {
        add_counters_by(game, card, "+1/+1", 2, Some(you));
    }
    Some(())
}
