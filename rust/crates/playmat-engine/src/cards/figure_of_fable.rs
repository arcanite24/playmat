//! Figure of Fable: "{G/W}: This creature becomes a Kithkin Scout with base power and toughness 2/3.
//! {1}{G/W}{G/W}: If this creature is a Scout, it becomes a Kithkin Soldier with base power and toughness 4/5.
//! {3}{G/W}{G/W}{G/W}: If this creature is a Soldier, it becomes a Kithkin Avatar with base power and toughness 7/8
//! and protection from each of your opponents."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("FigureOfFable.FigureOfFableScoutEffect", scout).effects(&[
        "new AddCardSubTypeSourceEffect(Duration.Custom, SubType.KITHKIN, SubType.SOLDIER)",
        "new SetBasePowerToughnessSourceEffect(4, 5, Duration.Custom)",
    ]),
    Handler::effect("FigureOfFable.FigureOfFableSoldierEffect", soldier).effects(&[
        "new AddCardSubTypeSourceEffect(Duration.Custom, SubType.KITHKIN, SubType.AVATAR)",
        "new SetBasePowerToughnessSourceEffect(7, 8, Duration.Custom)",
        "new GainAbilitySourceEffect(new ProtectionFromEachOpponentAbility(), Duration.Custom)",
    ]),
];

/// The source still on the battlefield and a Scout: the Soldier's subtypes and base 4/5 (`game.addEffect`, in order)
fn scout(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    becomes(game, d, cx, "Scout")
}

/// The same for a Soldier: an Avatar, base 7/8, protection from each opponent
fn soldier(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    becomes(game, d, cx, "Soldier")
}

fn becomes(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, is: &str) -> Option<()> {
    if !cx.source_on_battlefield(game) || !game.has_subtype(cx.source_id(), is) {
        return Some(());
    }
    cx.run(game, d, cx.effects);
    Some(())
}
