//! Saheeli, Filigree Master: "+1: Scry 1. You may tap an untapped artifact you control. If you do, draw a card.
//! −2: Create two 1/1 colorless Thopter artifact creature tokens with flying. They gain haste until end of turn.
//! −4: You get an emblem with "Artifact creatures you control get +1/+1" and "Artifact spells you cast cost {1}
//! less to cast.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SaheeliFiligreeMaster.SaheeliFiligreeMasterEffect", apply)
    .tokens(&["ThopterColorlessToken"])];

/// Two Thopters (`token.putOntoBattlefield(2, …)`), then haste until end of turn for the ones it made
/// (`GainAbilityTargetEffect` with `FixedTargets(token)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.battlefield.clone();
    cx.create_tokens(game, d, &cx.tokens[0], 2);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| !before.contains(o) && game.obj(*o).token).collect();
    for t in made {
        let zcc = game.obj(t).zcc;
        Arc::make_mut(&mut game.grants).push(crate::state::Grant {
            target: t,
            zcc,
            ability: AbilityDef::HasteAbility,
            until_end_of_turn: true,
            until_turn_of: None,
        });
    }
    Some(())
}
