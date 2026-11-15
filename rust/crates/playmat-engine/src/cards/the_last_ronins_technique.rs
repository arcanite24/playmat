//! The Last Ronin's Technique: "Sneak {1}{W}. Create three 1/1 white Ninja Turtle Spirit creature tokens. If this
//! spell's sneak cost was paid, they enter tapped and attacking."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheLastRoninsTechnique.TheLastRoninsTechniqueEffect", apply)
    .tokens(&["NinjaTurtleSpiritToken"])];

/// `SneakCondition`: the spell was cast for its sneak cost (the sneak spell's costs return an unblocked attacker);
/// three tokens, tapped and attacking when it was
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let sneaked = game.obj(cx.source_id()).def.spell.costs.contains(&CostDef::ReturnAttackerToHandTargetCost);
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: crate::card::Amount::Fixed(3),
        tapped_and_attacking: sneaked,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    };
    cx.run(game, d, &[e]);
    Some(())
}
