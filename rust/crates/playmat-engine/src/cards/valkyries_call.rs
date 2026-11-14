//! Valkyrie's Call: "Whenever a nontoken, non-Angel creature you control dies, return that card to the battlefield
//! under its owner's control with a +1/+1 counter on it. It has flying and is an Angel in addition to its other
//! types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ValkyriesCall.ValkyriesCallEffect", apply)];

/// The card, still in its graveyard, onto the battlefield under its owner's control: an Angel with flying from the
/// moment it enters (`ValkyriesCallContinuousEffect`, for its next zone change counter), then the +1/+1 counter
pub(super) fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for c in cx.objs(0) {
        if game.obj(c).zone != Zone::Graveyard || game.obj(c).token {
            continue;
        }
        let owner = game.obj(c).owner;
        let next = game.obj(c).zcc + 1;
        game.subtypes_while_there.push((c, next, "Angel".into()));
        Arc::make_mut(&mut game.grants).push(Grant {
            target: c,
            zcc: next,
            ability: AbilityDef::FlyingAbility,
            until_end_of_turn: false,
            until_turn_of: None,
        });
        enter_battlefield(game, d, c, owner, false);
        if d.stopped() {
            return None;
        }
        if game.obj(c).zone == Zone::Battlefield {
            add_counters_by(game, c, "+1/+1", 1, Some(cx.controller()));
        }
    }
    Some(())
}
