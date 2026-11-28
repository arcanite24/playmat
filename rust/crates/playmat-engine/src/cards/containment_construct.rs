//! Containment Construct: "Whenever you discard a card, you may exile that card from your graveyard. If you do, you
//! may play that card this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "ContainmentConstruct.ContainmentConstructTriggeredAbility",
        "new DiscardCardControllerTriggeredAbility(new ContainmentConstructEffect(), false)",
        in_graveyard,
    ),
    Handler::effect("ContainmentConstruct.ContainmentConstructEffect", apply),
];

/// `DISCARDED_CARD` by you (the base), the card in a graveyard as it triggers (Java checks it only then)
fn in_graveyard(game: &Game, t: &StackItem) -> bool {
    game.checking_item.is_none() || t.discarded.is_some_and(|c| game.obj(c).zone == Zone::Graveyard)
}

/// The discarded card (a `FixedTarget` of its id alone, so wherever it is now): "Do you want to …?" Yes exiles it,
/// playable this turn by you (`PlayFromNotOwnHandZoneTargetEffect(Duration.EndOfTurn)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.item.discarded else { return Some(()) };
    let you = cx.controller();
    if !choose_use(
        game,
        d,
        you,
        "Do you want to exile the discarded card?  You may play it this turn from exile.",
    )? {
        return Some(());
    }
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        let turn = game.turn;
        game.obj_mut(card).may_play = Some((you, turn));
    }
    Some(())
}

