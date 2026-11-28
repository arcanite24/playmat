//! Aetherspouts: "For each attacking creature, its owner puts it on the top or bottom of their library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Aetherspouts.AetherspoutsEffect", apply)];

/// From the active player on, each player is asked of each attacking creature they own, in battlefield order,
/// whether it goes on top (a token goes there without asking); then they order the ones going on top, one pick at a
/// time (the last one picked ends up topmost), and those going to the bottom the same way
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let active = game.active.unwrap_or(cx.controller());
    for k in 0..game.seats() {
        let p = (active + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let attacking: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).attacking.is_some() && game.has_type(*o, CardType::Creature))
            .filter(|o| game.obj(*o).owner == p)
            .collect();
        let (mut top, mut bottom) = (Vec::new(), Vec::new());
        for o in attacking {
            if game.obj(o).token {
                top.push(o);
                continue;
            }
            let name = log::object(game, o);
            let q = format!("Put {name} to the top? (else it goes to bottom)");
            let on_top = choose_use(game, d, p, &q)?;
            if log::kept(game) {
                let line = format!(
                    "{name} goes to the {} of {}'s library",
                    if on_top { "top" } else { "bottom" },
                    log::player(p)
                );
                log::inform(game, line);
            }
            if on_top { top.push(o) } else { bottom.push(o) }
        }
        for (cards, to_top) in [(top, true), (bottom, false)] {
            // the tokens first, then the cards in the order picked
            let (mut ordered, mut left): (Vec<ObjId>, Vec<ObjId>) = cards.into_iter().partition(|o| game.obj(*o).token);
            while left.len() > 1 {
                let picked = choose_cards(game, d, p, &left, &left, 1, 1, &item_rule(game, cx.item))?;
                let Some(o) = picked.first().copied() else { break };
                left.retain(|c| *c != o);
                ordered.push(o);
            }
            ordered.extend(left);
            for o in ordered {
                if game.obj(o).zone == Zone::Battlefield {
                    to_library(game, o, to_top);
                }
            }
        }
    }
    Some(())
}
