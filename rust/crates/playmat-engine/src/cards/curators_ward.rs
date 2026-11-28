//! Curator's Ward: "Enchant permanent. Enchanted permanent has hexproof. When enchanted permanent leaves the
//! battlefield, if it was historic, draw two cards. (Artifacts, legendaries, and Sagas are historic.)"

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "CuratorsWard.CuratorsWardTriggeredAbility",
    "new LeavesBattlefieldAllTriggeredAbility(new DrawCardSourceControllerEffect(2), StaticFilters.FILTER_PERMANENT)",
    enchanted_historic,
)];

/// `checkTrigger`: the permanent that left had this Aura attached (as it is, or as it last was on the battlefield)
/// and was historic (an artifact, legendary or a Saga) as it last was
fn enchanted_historic(game: &Game, t: &StackItem) -> bool {
    let Some(left) = t.left else { return false };
    let ward = game.obj(t.card);
    let on = ward.attached_to.or(ward.lki_attached_to).map(|(o, _)| o);
    on == Some(left)
        && (game.has_type(left, CardType::Artifact)
            || game.obj(left).def.supertypes.iter().any(|s| s == "LEGENDARY")
            || game.has_subtype(left, "Saga"))
}
