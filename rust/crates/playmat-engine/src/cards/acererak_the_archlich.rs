//! Acererak the Archlich: "When Acererak the Archlich enters, if you haven't completed Tomb of Annihilation, return
//! Acererak the Archlich to its owner's hand and venture into the dungeon. Whenever Acererak the Archlich attacks,
//! for each opponent, you create a 2/2 black Zombie creature token unless that player sacrifices a creature of their
//! choice."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("AcererakTheArchlich.AcererakTheArchlichCondition", not_completed_tomb),
    Handler::effect("AcererakTheArchlich.AcererakTheArchlichEffect", apply).tokens(&["ZombieToken"]),
];

fn not_completed_tomb(game: &Game, you: Seat, _: ObjId) -> bool {
    !game.player(you).completed_dungeons.iter().any(|d| d == "Tomb of Annihilation")
}

/// Each opponent chooses up to one creature to sacrifice (`TargetSacrifice(0, 1, …)` with `Player.choose`, asked even
/// with none); a Zombie for each who didn't sacrifice one
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut tokens = 0;
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        tokens += 1;
        let creatures: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
            .collect();
        let chosen = choose_cards(game, d, p, &creatures, &creatures, 0, 1, &rule)?;
        if let Some(&o) = chosen.first() {
            if game.obj(o).zone == Zone::Battlefield {
                sacrifice(game, o);
                tokens -= 1;
            }
        }
    }
    if tokens > 0 {
        cx.create_tokens(game, d, &cx.tokens[0], tokens);
    }
    Some(())
}
