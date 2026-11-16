//! Warden of the First Tree: "{1}{W/B}: Warden of the First Tree becomes a Human Warrior with base power and
//! toughness 3/3. {2}{W/B}{W/B}: If Warden of the First Tree is a Warrior, it becomes a Human Spirit Warrior with
//! trample and lifelink. {3}{W/B}{W/B}{W/B}: If Warden of the First Tree is a Spirit, put five +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("WardenOfTheFirstTree.WardenOfTheFirstTreeEffect", apply).effects(&[
        "new AddCardSubTypeSourceEffect(Duration.Custom, SubType.HUMAN, SubType.SPIRIT, SubType.WARRIOR)",
        "new GainAbilitySourceEffect(TrampleAbility.getInstance(), Duration.Custom)",
        "new GainAbilitySourceEffect(LifelinkAbility.getInstance(), Duration.Custom)",
    ])];

/// still on the battlefield and a Warrior: the subtypes, trample and lifelink, each lasting while it's there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) || !game.has_subtype(cx.source_id(), "Warrior") {
        return Some(());
    }
    cx.run(game, d, cx.effects);
    Some(())
}
