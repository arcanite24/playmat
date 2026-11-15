//! Innocent Traveler // Malicious Invader. Innocent Traveler: "At the beginning of your upkeep, any opponent may
//! sacrifice a creature. If no one does, transform Innocent Traveler." Malicious Invader: "Flying. Malicious Invader
//! gets +2/+0 as long as an opponent controls a Human."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InnocentTraveler.InnocentTravelerEffect", apply)];

/// Each opponent with a creature: "Sacrifice a creature?", then one chosen (`Player.choose`, not targeted) and
/// sacrificed; with none sacrificed, the Traveler transforms while it's still the permanent it was
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    let mut sacrificed = false;
    for p in (1..n).map(|i| (you + i) % n) {
        if game.player(p).lost {
            continue;
        }
        let creatures: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
            .collect();
        if creatures.is_empty() || !choose_use(game, d, p, "Sacrifice a creature?")? {
            continue;
        }
        let rule = item_rule(game, cx.item);
        let Some(o) = choose_cards(game, d, p, &creatures, &creatures, 1, 1, &rule)?.first().copied() else {
            continue;
        };
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        sacrifice(game, o);
        if game.obj(o).zone != Zone::Battlefield {
            sacrificed = true;
        }
    }
    if !sacrificed && cx.source_on_battlefield(game) {
        transform(game, cx.source_id());
    }
    Some(())
}
