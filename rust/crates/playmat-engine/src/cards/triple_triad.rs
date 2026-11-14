//! Triple Triad: "At the beginning of your upkeep, each player exiles the top card of their library. Until end of
//! turn, you may play the card you own exiled this way and each other card exiled this way with lesser mana value
//! than it without paying their mana costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TripleTriad.TripleTriadEffect", apply)];

/// Each player in range in turn from you (`getPlayersInRange`) exiles their top card; with one of yours among them,
/// it and those of lesser mana value may be played by you this turn without paying their mana costs
/// (`PlayFromNotOwnHandZoneTargetEffect`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut exiled: Vec<ObjId> = Vec::new();
    let n = game.seats();
    for p in (0..n).map(|i| (you + i) % n) {
        if game.player(p).lost {
            continue;
        }
        let Some(card) = game.player(p).library.first().copied() else { continue };
        game.move_to(card, Zone::Exile);
        exiled.push(card);
    }
    let Some(&yours) = exiled.iter().find(|c| game.obj(**c).owner == you) else { return Some(()) };
    let mv = game.obj(yours).def.mana_value();
    let turn = game.turn;
    for c in exiled {
        if (c == yours || game.obj(c).def.mana_value() < mv) && game.obj(c).zone == Zone::Exile {
            let o = game.obj_mut(c);
            o.may_play = Some((you, turn));
            o.play_free = true;
        }
    }
    Some(())
}
