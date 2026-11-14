//! Professor Onyx: "−3: Each opponent sacrifices a creature with the greatest power among creatures that player
//! controls. −8: Each opponent may discard a card. If they don't, they lose 3 life. Repeat this process six more
//! times."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("ProfessorOnyx.ProfessorOnyxPredicate", greatest_power),
    Handler::effect("ProfessorOnyx.ProfessorOnyxEffect", apply),
];

/// its power is at least that of each creature its controller controls
fn greatest_power(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let controller = game.obj(id).controller;
    let max = game
        .battlefield
        .iter()
        .filter(|c| game.obj(**c).controller == controller && game.has_type(**c, CardType::Creature))
        .map(|c| game.power_toughness(*c).0)
        .max()
        .unwrap_or(i32::MIN);
    game.power_toughness(id).0 >= max
}

/// Six rounds (Java's loop, not seven): each opponent in turn order picks up to one card to discard
/// (`TargetDiscard(0, 1)` with `Player.choose`), then each discards it or loses 3 life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    for _ in 0..6 {
        let mut picks: Vec<(Seat, Option<ObjId>)> = Vec::new();
        for &p in &opponents {
            let hand = game.player(p).hand.clone();
            let pick = if hand.is_empty() { None } else { choose_cards(game, d, p, &hand, &hand, 0, 1, "discard")?.first().copied() };
            picks.push((p, pick));
        }
        for (p, pick) in picks {
            match pick {
                Some(c) if game.obj(c).zone == Zone::Hand => {
                    discard_card(game, p, c);
                    game.discarded_cards();
                }
                _ => lose_life(game, p, 3),
            }
        }
    }
    Some(())
}
