//! Collective Effort: "Put a +1/+1 counter on each creature target player controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CollectiveEffort.CollectiveEffortEffect", apply)];

/// A counter on each creature the target player controls (`source.getFirstTarget()`, the player of its mode)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let player = cx.legal.iter().flatten().find_map(|t| if let Target::Player(p) = t { Some(*p) } else { None })?;
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == player && game.has_type(*o, CardType::Creature))
        .collect();
    for c in creatures {
        game.obj_mut(c).add_counters("+1/+1", 1);
        counters_put_triggers(game, c, "+1/+1", 1, Some(cx.controller()));
    }
    Some(())
}
