//! Tragic Arrogance: "For each player, you choose from among the permanents that player controls an artifact, a
//! creature, an enchantment, and a planeswalker. Then each player sacrifices all other nonland permanents they
//! control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TragicArrogance.TragicArroganceEffect", apply)];

/// For each player from you in turn order, you choose (`chooseTarget`, one of each kind when there's one to choose)
/// an artifact, a creature, an enchantment and a planeswalker they control; then each player, in the same order,
/// sacrifices their other nonland permanents
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let players: Vec<Seat> =
        (0..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    let rule = item_rule(game, cx.item);
    let mut chosen: Vec<ObjId> = Vec::new();
    for p in &players {
        for t in [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Planeswalker] {
            let possible: Vec<ObjId> = game
                .battlefield
                .iter()
                .copied()
                .filter(|o| game.obj(*o).controller == *p && game.has_type(*o, t))
                .collect();
            if possible.is_empty() {
                continue;
            }
            chosen.extend(choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?);
        }
    }
    for p in &players {
        let doomed: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == *p && !game.has_type(*o, CardType::Land) && !chosen.contains(o))
            .collect();
        for o in doomed {
            if game.obj(o).zone == Zone::Battlefield {
                sacrifice(game, o);
            }
        }
    }
    Some(())
}
