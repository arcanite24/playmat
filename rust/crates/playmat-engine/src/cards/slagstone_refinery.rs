//! Slagstone Refinery: "Whenever Slagstone Refinery or another nontoken artifact you control is put into a graveyard
//! from the battlefield or is put into exile from the battlefield, create a tapped Powerstone token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SlagstoneRefinery.SlagstoneRefineryTriggeredAbility",
    "new LeavesBattlefieldAllTriggeredAbility(new CreateTokenEffect(new PowerstoneToken(), 1, true), StaticFilters.FILTER_CONTROLLED_PERMANENT_ARTIFACT)",
    to_graveyard_or_exile,
)];

/// the permanent that left (an artifact you controlled: the base's filter, as it left) went to a graveyard or
/// exile, and it's this or isn't a token
fn to_graveyard_or_exile(game: &Game, item: &StackItem) -> bool {
    let Some(left) = item.left else { return false };
    let o = game.obj(left);
    matches!(o.zone, Zone::Graveyard | Zone::Exile) && (left == item.card || !o.token)
}
