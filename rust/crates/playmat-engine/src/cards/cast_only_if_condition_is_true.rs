//! `CastOnlyIfConditionIsTrueAbility(condition[, text])`: "cast this spell only if a creature died this turn"
//! (Grim Wanderer's Tragic Backstory). A `Zone.ALL` static whose `CastOnlyIfConditionIsTrueEffect` stops the
//! card's own cast (`CAST_SPELL`) while the condition doesn't hold. The transpiler passes the condition as a
//! `ConditionalOneShotEffect` of the static's.

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("CastOnlyIfConditionIsTrueEffect", Rule::CantCast(cant_cast)).in_any_zone()];

fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> bool {
    if id != cx.source {
        return false;
    }
    let Some(EffectDef::ConditionalOneShotEffect { condition, .. }) = cx.effects.first() else { return false };
    !game.condition_holds(Some(condition), seat, id)
}
