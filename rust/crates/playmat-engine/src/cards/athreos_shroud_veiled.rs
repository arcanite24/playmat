//! Athreos, Shroud-Veiled: "Indestructible. As long as your devotion to white and black is less than seven, Athreos
//! isn't a creature. At the beginning of your end step, put a coin counter on another target creature. Whenever a
//! creature with a coin counter on it dies or is put into exile, return that card to the battlefield under your
//! control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "AthreosShroudVeiled.AthreosShroudVeiledTriggeredAbility",
        "new LeavesBattlefieldAllTriggeredAbility(new AthreosShroudVeiledEffect(), StaticFilters.FILTER_PERMANENT_CREATURE)",
        coin_creature_died_or_exiled,
    ),
    // the trigger's effect at the card that left (`MageObjectReference` of the permanent)
    Handler::effect("AthreosShroudVeiled.AthreosShroudVeiledEffect", apply),
];

/// the creature that left had a coin counter on it as it left, and went to a graveyard or exile
fn coin_creature_died_or_exiled(game: &Game, item: &StackItem) -> bool {
    let Some(left) = item.left else { return false };
    matches!(game.obj(left).zone, Zone::Graveyard | Zone::Exile)
        && game.obj(left).lki_counters.iter().any(|(c, n)| c == "coin" && *n > 0)
}

/// the card, still where it went (a token isn't a card), onto the battlefield under your control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.item.left else { return Some(()) };
    if !matches!(game.obj(card).zone, Zone::Graveyard | Zone::Exile) || game.obj(card).token {
        return Some(());
    }
    put_onto_battlefield(game, d, card, cx.controller());
    if d.stopped() {
        return None;
    }
    Some(())
}
