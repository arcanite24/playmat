//! Imaginary Threats: "Creatures target opponent controls attack this turn if able. During that player's next untap
//! step, creatures they control don't untap."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ImaginaryThreats.ImaginaryThreatsEffect", apply)
    .filters(&["StaticFilters.FILTER_OPPONENTS_PERMANENT_CREATURE"])];

/// `AttacksIfAbleAllEffect(creatures that player controls, EndOfTurn)` added for the turn (in a duel the target
/// opponent is the only one, so "creatures your opponents control")
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.player(0).is_none() {
        return Some(());
    }
    let filter = cx.filters.first().cloned().unwrap_or_default();
    let e = EffectDef::AttacksIfAbleAllEffect { filter, this_turn: true };
    game.eot_effects.push((e, cx.controller(), cx.source_id()));
    Some(())
}
