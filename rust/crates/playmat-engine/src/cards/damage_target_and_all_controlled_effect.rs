//! `DamageTargetAndAllControlledEffect(first, second, filter)`: "deals 3 damage to target player and 1 damage to
//! each creature that player controls" (Radiating Lightning); `(amount)` is the same amount to each creature

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DamageTargetAndAllControlledEffect", apply)
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])
    .args(&["int", "int", "filter"])];

/// The target (a permanent, else a player) first; then each permanent of the filter that the target player (or the
/// target permanent's controller) controls, in battlefield order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let first = cx.arg_int(0)?;
    let second = cx.arg_int(1).unwrap_or(first);
    let filter = cx.filters.last()?;
    let target = cx.target(0)?;
    let player = match target {
        Target::Obj(o) if game.obj(o).zone == Zone::Battlefield => {
            deal_damage(game, cx.source_id(), target, first, false);
            Some(game.obj(o).controller)
        }
        Target::Player(p) => {
            deal_damage(game, cx.source_id(), target, first, false);
            Some(p)
        }
        _ => None,
    };
    let Some(p) = player else { return Some(()) };
    let all: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.filter_matches(filter, p, Some(cx.source_id()), *o))
        .collect();
    for o in all {
        if game.obj(o).zone == Zone::Battlefield {
            deal_damage(game, cx.source_id(), Target::Obj(o), second, false);
        }
    }
    Some(())
}
