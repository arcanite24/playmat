//! `CantBeTargetedSourceEffect(filter, Duration.WhileOnBattlefield)`: "{this} can't be the target of nongreen spells
//! or abilities from nongreen sources" (Gaea's Revenge)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("CantBeTargetedSourceEffect", Rule::CantBeTargeted(by_filter)).args(&["filter", "while_on_battlefield"])];

/// `TARGET` of this permanent by a spell or ability whose stack object (a spell) or source object (an ability's
/// permanent or card: `StackAbility.getSourceObject`) matches the filter
fn by_filter(game: &Game, cx: &StaticCx, _seat: Seat, id: ObjId, src: ObjId) -> bool {
    id == cx.source && cx.matches(game, 0, src)
}
