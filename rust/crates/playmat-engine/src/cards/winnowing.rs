//! Winnowing: "For each player, you choose a creature that player controls. Then each player sacrifices all other
//! creatures they control that don't share a creature type with the chosen creature they control."

use super::invasion_of_new_capenna::share_creature_types;
use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Winnowing.WinnowingEffect", apply)];

/// Each player in range in turn from you (`getPlayersInRange`), with a creature: you choose one of theirs (not targeted), and
/// the rest of theirs that don't share a creature type with it are gathered; then all of them are sacrificed.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut doomed: Vec<ObjId> = Vec::new();
    let n = game.seats();
    for p in (0..n).map(|i| (you + i) % n) {
        if game.player(p).lost {
            continue;
        }
        let theirs: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
            .collect();
        if theirs.is_empty() {
            continue;
        }
        let Some(&kept) = choose_cards(game, d, you, &theirs, &theirs, 1, 1, "a creature to keep")?.first() else {
            continue;
        };
        for o in theirs {
            if o != kept && !share_creature_types(game, o, kept) && !doomed.contains(&o) {
                doomed.push(o);
            }
        }
    }
    for o in doomed {
        sacrifice(game, o);
    }
    Some(())
}
