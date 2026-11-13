//! Niv-Mizzet Reborn: "When Niv-Mizzet Reborn enters, reveal the top ten cards of your library. For each color pair,
//! choose a card that's exactly those colors from among them. Put the chosen cards into your hand and the rest on the
//! bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NivMizzetReborn.NivMizzetRebornEffect", apply)];

fn colors(game: &Game, c: ObjId) -> Vec<Color> {
    let mut v = game.obj(c).def.colors.clone();
    v.sort_by_key(|x| *x as u8);
    v.dedup();
    v
}

/// `NivMizzetRebornTarget` (0 to 10 two-colored cards, no two of the same colors) in one `choose`: the cards that
/// still fit are offered after each pick, until none does or the player is done
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(10).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let left: Vec<ObjId> = cards
            .iter()
            .copied()
            .filter(|c| {
                !chosen.contains(c)
                    && colors(game, *c).len() == 2
                    && chosen.iter().all(|x| colors(game, *x) != colors(game, *c))
            })
            .collect();
        if !chosen.is_empty() && left.is_empty() {
            break;
        }
        match ask(game, d, you, &Query::Cards { objs: left.clone(), required: false }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if left.contains(&o) => chosen.push(o),
            Answer::Stop => return None,
            Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
            _ => break,
        }
        if d.stopped() {
            return None;
        }
    }
    for c in &chosen {
        game.move_to(*c, Zone::Hand);
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    Some(())
}
