//! `CastAsThoughItHadFlashIfConditionAbility(condition, rule)`: "you may cast this spell as though it had flash if
//! you control a Human" (Bard's Company, Illusion Spinners, Silver Scrutiny). A `Zone.ALL` static of two effects:
//! `CastAsThoughItHadFlashSourceEffect` (the card may always be cast as though it had flash) and
//! `CantFlashUnlessConditionEffect` (`CAST_SPELL_LATE`: with the spell on the stack, unless it's its controller's
//! main phase with nothing else on the stack, the cast is undone when the condition doesn't hold). The transpiler
//! passes the condition as the second effect's `ConditionalAsThoughEffect`.

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[
    Handler::as_though("CastAsThoughItHadFlashIfConditionAbility", AsThough::Flash(flash)).in_any_zone(),
    Handler::rule("CantFlashUnlessConditionEffect", Rule::CantCastOnStack(cant_flash)).in_any_zone(),
];

fn flash(_: &Game, cx: &StaticCx, _: Seat, id: ObjId, _: &CardDef) -> bool {
    id == cx.source
}

fn cant_flash(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> bool {
    if id != cx.source {
        return false;
    }
    if game.step.is_some_and(|s| s.is_main()) && game.active == Some(seat) && game.stack.len() == 1 {
        return false;
    }
    let Some(EffectDef::ConditionalAsThoughEffect { condition, .. }) = cx.effects.first() else { return false };
    !game.condition_holds(Some(condition), seat, id)
}
