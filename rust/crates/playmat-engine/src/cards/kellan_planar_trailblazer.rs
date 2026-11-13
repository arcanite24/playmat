//! Kellan, Planar Trailblazer: "{1}{R}: If Kellan is a Scout, it becomes a Human Faerie Detective and gains
//! 'Whenever Kellan deals combat damage to a player, exile the top card of your library. You may play that card this
//! turn.' {2}{R}: If Kellan is a Detective, it becomes a 3/2 Human Faerie Rogue and gains double strike."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("KellanPlanarTrailblazer.KellanPlanarTrailblazerDetectiveEffect", detective).effects(&[
        "new AddCardSubTypeSourceEffect(Duration.Custom, SubType.HUMAN, SubType.FAERIE, SubType.DETECTIVE)",
        r#"new GainAbilitySourceEffect(new DealsCombatDamageToAPlayerTriggeredAbility(new ExileTopXMayPlayUntilEffect(1, Duration.EndOfTurn).withTextOptions("that card", true)), Duration.Custom)"#,
    ]),
    Handler::effect("KellanPlanarTrailblazer.KellanPlanarTrailblazerRogueEffect", rogue).effects(&[
        "new AddCardSubTypeSourceEffect(Duration.Custom, SubType.HUMAN, SubType.FAERIE, SubType.ROGUE)",
        "new SetBasePowerToughnessSourceEffect(3, 2, Duration.Custom)",
        "new GainAbilitySourceEffect(DoubleStrikeAbility.getInstance(), Duration.Custom)",
    ]),
];

/// `getSourcePermanentIfItStillExists` with the subtype, then the continuous effects in Java's order
fn becomes(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, subtype: &str) -> Option<()> {
    if !cx.source_on_battlefield(game) || !game.has_subtype(cx.source_id(), subtype) {
        return Some(());
    }
    cx.run(game, d, cx.effects);
    if d.stopped() {
        return None;
    }
    Some(())
}

fn detective(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    becomes(game, d, cx, "Scout")
}

fn rogue(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    becomes(game, d, cx, "Detective")
}
