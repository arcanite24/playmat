//! Woodcaller Automaton: "When Woodcaller Automaton enters the battlefield, if you cast it, untap target land you
//! control. It becomes a Treefolk creature with haste and base power and toughness equal to Woodcaller Automaton's
//! power and toughness. It's still a land."

use super::*;
use crate::card::{Amount, Value};

pub const HANDLERS: &[Handler] = &[Handler::effect("WoodcallerAutomaton.WoodcallerAutomatonEffect", apply).effects(&[
    r#"new BecomesCreatureTargetEffect(new CreatureToken(0,0,"",SubType.TREEFOLK).withAbility(HasteAbility.getInstance()), false, true, Duration.Custom)"#,
])];

/// The source's power and toughness now (`SourcePermanentPowerValue.ALLOW_NEGATIVE`, as it last was once it's gone)
/// make the token's, then `BecomesCreatureTargetEffect` on the target land for good
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let power = game.value(&Amount::Of(Value::SourcePower), you, me, cx.item.x);
    let toughness = game.value(&Amount::Of(Value::SourceToughness), you, me, cx.item.x);
    let mut e = cx.effects.first()?.clone();
    if let EffectDef::BecomesCreatureTargetEffect { token, .. } = &mut e {
        token.power = power;
        token.toughness = toughness;
    }
    cx.run(game, d, &[e]);
    Some(())
}
