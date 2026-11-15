//! Anax, Hardened in the Forge: "Anax's power is equal to your devotion to red. Whenever Anax or another nontoken
//! creature you control dies, create a 1/1 red Satyr creature token with "This creature can't block." If the creature
//! had power 4 or greater, create two of those tokens instead."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "AnaxHardenedInTheForge.AnaxHardenedInTheForgeTriggeredAbility",
        "new DiesThisOrAnotherTriggeredAbility(/* $0 */ new AnaxHardenedInTheForgeTokensEffect(), $1, $2)",
        always,
    ),
    Handler::effect("AnaxHardenedInTheForge.AnaxHardenedInTheForgeTokensEffect", apply).tokens(&["SatyrCantBlockToken"]),
];

/// the base trigger's event and filter are the whole check
fn always(_: &Game, _: &StackItem) -> bool {
    true
}

/// two Satyrs if the creature that died had power 4 or greater as it last was (`zEvent.getTarget().getPower()`),
/// else one
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let power = cx.item.died.and_then(|(c, _)| game.obj(c).power_lki).unwrap_or(0);
    let n = if power > 3 { 2 } else { 1 };
    cx.create_tokens(game, d, &cx.tokens[0], n);
    Some(())
}
