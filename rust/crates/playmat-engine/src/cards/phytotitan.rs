//! Phytotitan: "When Phytotitan dies, return it to the battlefield tapped under its owner's control at the beginning
//! of their next upkeep."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Phytotitan.PhytotitanEffect", apply)];

/// `AtTheBeginOfPlayersNextUpkeepDelayedTriggeredAbility(ReturnToBattlefieldUnderOwnerControlTargetEffect(tapped),
/// owner)` on the card that died (the trigger's `SetTargetPointer.CARD`), while it's still that card
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0).or_else(|| {
        let s = cx.source_id();
        (game.obj(s).zone == Zone::Graveyard).then_some(s)
    })?;
    let owner = game.obj(card).owner;
    let delayed = Delayed {
        rule: "Return it to the battlefield tapped under its owner's control at the beginning of their next upkeep."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: vec![(card, game.obj(card).zcc)],
        targets: Vec::new(),
        condition: None,
    };
    game.at_first_upkeep.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), owner) });
    Some(())
}
