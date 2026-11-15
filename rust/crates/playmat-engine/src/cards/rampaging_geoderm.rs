//! Rampaging Geoderm: "Whenever you attack, target attacking creature gets +1/+1 until end of turn. If it's attacking
//! a battle, put a +1/+1 counter on it instead."

use super::*;
use crate::card::Duration;

pub const HANDLERS: &[Handler] = &[Handler::effect("RampagingGeoderm.RampagingGeodermEffect", apply)];

/// attacking a battle: a +1/+1 counter; else +1/+1 until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    let battle = game.obj(creature).attacking_pw.is_some_and(|(b, _)| game.has_type(b, CardType::Battle));
    if battle {
        add_counters_by(game, creature, "+1/+1", 1, Some(cx.controller()));
        return Some(());
    }
    let boost =
        EffectDef::BoostTargetEffect { power: Amount::Fixed(1), toughness: Amount::Fixed(1), duration: Duration::EndOfTurn };
    cx.run_on(game, d, &[boost], vec![Target::Obj(creature)]);
    (!d.stopped()).then_some(())
}
