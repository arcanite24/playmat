//! `CreaturesAreExiledOnDeathReplacementEffect`: "If a creature an opponent controls would die, exile it instead."
//! (Liesa, Forgotten Archangel; Vren, the Relentless; Misery's Shadow; Garruk, Veiled Butcher)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "CreaturesAreExiledOnDeathReplacementEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)
.args(&["filter"])];

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    // `isDiesEvent`: from the battlefield to a graveyard
    let Some(id) = ev.target else { return false };
    ev.from == Some(Zone::Battlefield) && ev.to == Some(Zone::Graveyard) && cx.matches(game, 0, id)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
