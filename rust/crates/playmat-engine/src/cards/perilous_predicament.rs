//! Perilous Predicament: "Each opponent sacrifices an artifact creature and a nonartifact creature."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("PerilousPredicament.PerilousPredicamentSacrificeOpponentsEffect", apply)];

/// Each opponent in turn order chooses an artifact creature, then a nonartifact creature (`TargetSacrifice`, each
/// asked only when they control one); then every chosen permanent still on the battlefield is sacrificed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut chosen = Vec::new();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        for artifact in [true, false] {
            let creatures: Vec<ObjId> = game
                .battlefield
                .iter()
                .copied()
                .filter(|o| {
                    game.obj(*o).controller == p
                        && game.has_type(*o, CardType::Creature)
                        && game.has_type(*o, CardType::Artifact) == artifact
                })
                .collect();
            if creatures.is_empty() {
                continue;
            }
            chosen.extend(choose_cards(game, d, p, &creatures, &creatures, 1, 1, &rule)?);
        }
    }
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    Some(())
}
