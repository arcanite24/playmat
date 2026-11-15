//! Cataclysmic Gearhulk: "Vigilance. When Cataclysmic Gearhulk enters, each player chooses an artifact, a creature,
//! an enchantment, and a planeswalker from among the nonland permanents they control, then sacrifices the rest."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CataclysmicGearhulk.CataclysmicGearhulkEffect", apply)];

/// the players still in the game, from `you` in turn order (`getPlayersInRange`)
fn in_range(game: &Game, you: Seat) -> Vec<Seat> {
    let n = game.seats();
    (0..n).map(|i| (you + i) % n).filter(|p| !game.player(*p).lost).collect()
}

/// Each player in turn, for artifact, creature, enchantment and planeswalker: a not-targeted
/// `TargetControlledPermanent(1, 1, nonland <type>)` chosen (`chooseTarget` until one is) when they control one;
/// then every nonland permanent on the battlefield that wasn't chosen is sacrificed, in battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let rule = item_rule(game, cx.item);
    let mut chosen: Vec<ObjId> = Vec::new();
    for p in in_range(game, cx.controller()) {
        for t in [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Planeswalker] {
            let possible: Vec<ObjId> = game
                .battlefield
                .iter()
                .copied()
                .filter(|o| {
                    game.obj(*o).controller == p && game.has_type(*o, t) && !game.has_type(*o, CardType::Land)
                })
                .collect();
            if possible.is_empty() {
                continue;
            }
            let picked = choose_cards(game, d, p, &possible, &possible, 1, 1, &rule)?;
            if let Some(o) = picked.first() {
                if game.obj(*o).zone == Zone::Battlefield {
                    chosen.push(*o);
                }
            }
        }
    }
    let all: Vec<ObjId> = game.battlefield.iter().copied().collect();
    for o in all {
        if !chosen.contains(&o) && !game.has_type(o, CardType::Land) {
            sacrifice(game, o);
        }
    }
    Some(())
}
