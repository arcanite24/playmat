//! Crackling Doom: "Crackling Doom deals 2 damage to each opponent. Each opponent sacrifices a creature with the
//! greatest power among creatures that player controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CracklingDoom.CracklingDoomEffect", apply)];

/// Each opponent in turn order chooses one of their creatures with the greatest power among theirs
/// (`TargetPermanent(filter)`, not targeted); then the chosen ones are sacrificed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut chosen = Vec::new();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let creatures: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
            .collect();
        let Some(most) = creatures.iter().map(|o| game.power_toughness(*o).0).max() else { continue };
        let biggest: Vec<ObjId> =
            creatures.iter().copied().filter(|o| game.power_toughness(*o).0 == most).collect();
        let rule = item_rule(game, cx.item);
        if let Some(o) = choose_cards(game, d, p, &biggest, &biggest, 1, 1, &rule)?.first() {
            chosen.push(*o);
        }
    }
    for o in chosen {
        sacrifice(game, o);
    }
    Some(())
}
