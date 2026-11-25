//! Jace, Cunning Castaway: "+1: Whenever one or more creatures you control deal combat damage to a player this turn,
//! draw a card, then discard a card. −2: Create a 2/2 blue Illusion creature token with "When this creature becomes
//! the target of a spell, sacrifice it." −5: Create two tokens that are copies of Jace, Cunning Castaway, except
//! they're not legendary."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "JaceCunningCastaway.JaceCunningCastawayDamageTriggeredAbility",
        "new OneOrMoreCombatDamagePlayerTriggeredAbility(new DrawDiscardControllerEffect(1, 1), StaticFilters.FILTER_CONTROLLED_CREATURES)",
        any_batch,
    ),
    Handler::effect("JaceCunningCastaway.JaceCunningCastawayCopyEffect", copies),
];

/// `CreateTokenCopyTargetEffect(you, null, false, 2)` with `setIsntLegendary(true)`, of this permanent or as it last
/// was (`getPermanentOrLKIBattlefield`)
fn copies(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateTokenCopyTargetEffect {
        number: Some(2),
        becomes_artifact: false,
        not_legendary: true,
        add_types: Vec::new(),
        haste: false,
        flying: false,
        power: None,
        toughness: None,
        only_subtype: None,
        add_subtypes: Vec::new(),
        only_color: Vec::new(),
        abilities: Vec::new(),
        legendary: false,
        tapped: false,
        attacking: false,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(cx.source_id())]);
    Some(())
}
