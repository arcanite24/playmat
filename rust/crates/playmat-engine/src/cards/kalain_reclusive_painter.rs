//! Kalain, Reclusive Painter: "When Kalain, Reclusive Painter enters, create a Treasure token. Other creatures you
//! control enter with an additional +1/+1 counter on them for each mana from a Treasure spent to cast them."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "KalainReclusivePainter.KalainReclusivePainterEffect",
    &[EventType::EntersBattlefield],
    applies,
    replace,
)];

/// another creature entering under your control
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    id != cx.source && ev.player == cx.controller && game.has_type(id, CardType::Creature)
}

/// a +1/+1 counter for each mana from a Treasure spent to cast it (`ManaPaidSourceWatcher.getTreasurePaid`)
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let Some(id) = ev.target else { return false };
    let n = game.obj(id).treasure_spent as i32;
    if n > 0 {
        add_counters_by(game, id, "+1/+1", n, Some(cx.controller));
    }
    false
}
