//! Fblthp, the Lost: "When Fblthp, the Lost enters, draw a card. If it entered from your library or was cast from
//! your library, draw two cards instead. When Fblthp becomes the target of a spell, shuffle Fblthp into its owner's
//! library."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "FblthpTheLost.FblthpTheLostTriggeredAbility",
        "new EntersBattlefieldTriggeredAbility(new FblthpTheLostDrawEffect())",
        entered,
    ),
    // (the trigger's `checkTrigger` swaps in `DrawCardSourceControllerEffect(2)` or `(1)`: not a class of the card)
    Handler::effect("FblthpTheLost.FblthpTheLostDrawEffect", apply),
];

/// its entering: the base trigger is all of it
fn entered(_: &Game, _: &StackItem) -> bool {
    true
}

/// two cards when it entered from the library (`getFromZone() == LIBRARY`) or as a spell cast from the library
/// (`FblthpTheLostWatcher`), else one
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = game.obj(cx.source_id());
    let from_library = match o.entered_from {
        Some(Zone::Library) => true,
        Some(Zone::Stack) => o.cast_from == Some(Zone::Library),
        _ => false,
    };
    draw(game, cx.controller(), if from_library { 2 } else { 1 });
    Some(())
}
