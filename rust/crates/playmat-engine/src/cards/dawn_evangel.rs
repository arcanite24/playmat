//! Dawn Evangel: "Whenever a creature dies, if an Aura you controlled was attached to it, return target creature card
//! with mana value 2 or less from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DawnEvangel.DawnEvangelAbility",
    "new DiesCreatureTriggeredAbility(new ReturnFromGraveyardToHandTargetEffect(), false)",
    aura_attached,
)];

/// the creature that died had an Aura you control attached to it as it died (its attachments as it last was on the
/// battlefield, each as it last was: `getPermanentOrLKIBattlefield`)
fn aura_attached(game: &Game, item: &StackItem) -> bool {
    let Some((died, _)) = item.died else { return false };
    game.obj(died).lki_attachments.iter().any(|(a, _)| {
        let o = game.obj(*a);
        o.def.subtypes.iter().any(|s| s == "Aura") && o.controller == item.controller
    })
}
