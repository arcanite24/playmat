//! Teferi, Temporal Pilgrim's −12: "Target opponent chooses a permanent they control and returns it to its owner's
//! hand. Then they shuffle each nonland permanent they control into its owner's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TeferiTemporalPilgrim.TeferiTemporalPilgrimEffect", apply)];

/// `chooseTarget(TargetControlledPermanent)` (not targeted, one required), then every nonland permanent they control
/// to its owner's library, and each owner shuffles
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let theirs: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == p).collect();
    if !theirs.is_empty() {
        let rule = item_rule(game, cx.item);
        if let Some(&o) = choose_cards(game, d, p, &theirs, &theirs, 1, 1, &rule)?.first() {
            game.move_to(o, Zone::Hand);
        }
    }
    let nonland: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && !game.has_type(*o, CardType::Land))
        .collect();
    let mut owners: Vec<Seat> = Vec::new();
    for o in nonland {
        let owner = game.obj(o).owner;
        if !owners.contains(&owner) {
            owners.push(owner);
        }
        game.move_to(o, Zone::Library);
    }
    for owner in owners {
        let mut library = game.player(owner).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(owner).library = library;
    }
    Some(())
}
