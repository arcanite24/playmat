//! Tocasia, Dig Site Mentor: "Creatures you control have vigilance and "{T}: Surveil 1." {2}{G}{G}{W}{W}{U}{U},
//! Exile Tocasia, Dig Site Mentor from your graveyard: Return any number of target artifact cards with total mana
//! value 10 or less from your graveyard to the battlefield. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "TocasiaDigSiteMentor.TocasiaDigSiteMentorTarget",
    "new TargetCardInYourGraveyard(0, Integer.MAX_VALUE, filterStatic, false)",
    can_target,
)];

/// `CardUtil.checkCanTargetTotalValueLimit(…, MageObject::getManaValue, 10)`
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    super::joshua_phoenixs_dominant::within_total_mana_value(game, id, 10)
}
