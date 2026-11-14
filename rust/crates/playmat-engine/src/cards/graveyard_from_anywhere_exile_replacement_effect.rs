//! `GraveyardFromAnywhereExileReplacementEffect(filter, onlyYou)`: "If an instant or sorcery card would be put into a
//! graveyard from anywhere, exile it instead." (Dryad Militant)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "GraveyardFromAnywhereExileReplacementEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)
.args(&["filter", "text"])];

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    let o = game.obj(id);
    // a card (`game.getCard`: not a token), of the filter, yours with `onlyYou`
    ev.to == Some(Zone::Graveyard)
        && !o.token
        && (!cx.arg_bool(1) || o.owner == cx.controller)
        && cx.matches(game, 0, id)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
