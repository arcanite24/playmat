//! Rip, Spawn Hunter: "Survival — At the beginning of your second main phase, if Rip is tapped, reveal the top X
//! cards of your library, where X is its power. Put any number of creature and/or Vehicle cards with different powers
//! from among them into your hand. Put the rest on the bottom of your library in a random order."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] = &[Handler::effect("RipSpawnHunter.RipSpawnHunterEffect", apply)];

/// Rip's power (as it last was); below 1, nothing. The top that many revealed, then `HumanPlayer.choose` of
/// `RipSpawnHunterTarget` (0 to any creature or Vehicle cards): what can be picked reads the filter alone, but a pick
/// whose power one already chosen has isn't added; then the chosen into the hand together and the rest on the bottom
/// in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let power = game.power_toughness(cx.source_id()).0;
    if power < 1 {
        return Some(());
    }
    let top: Vec<ObjId> = game.player(you).library.iter().take(power as usize).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc: cx.source.1,
        paren: false,
        cards: top.iter().map(|c| (*c, game.obj(*c).zone)).collect(),
    });
    let possible: Vec<ObjId> = top
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Creature) || game.obj(*c).def.has_subtype("Vehicle"))
        .collect();
    let mut chosen: Vec<ObjId> = Vec::new();
    if !possible.is_empty() {
        loop {
            let left: Vec<ObjId> = possible.iter().copied().filter(|c| !chosen.contains(c)).collect();
            match ask(game, d, you, &Query::Cards { objs: left.clone(), required: false }) {
                Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                Answer::Obj(o) if left.contains(&o) => {
                    let p = game.obj(o).def.power;
                    if chosen.iter().all(|c| game.obj(*c).def.power != p) {
                        chosen.push(o);
                        if chosen.len() == possible.len() {
                            break;
                        }
                    }
                }
                Answer::Stop => return None,
                Answer::Bool(_) => break,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        }
    }
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, PutCards::Hand) {
        return None;
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_cards(game, d, you, &rest, PutCards::BottomRandom) {
        return None;
    }
    Some(())
}
