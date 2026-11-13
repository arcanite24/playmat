//! Nine-Lives Familiar: "When this creature dies, if it had a revival counter on it, return it to the battlefield
//! with one fewer revival counter on it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("NineLivesFamiliar.NineLivesFamiliarCondition", had_revival),
    Handler::effect("NineLivesFamiliar.NineLivesFamiliarEffect", apply),
];

fn revival(game: &Game, source: ObjId) -> i32 {
    game.obj(source).lki_counters.iter().find(|(n, _)| n == "revival").map_or(0, |(_, k)| *k)
}

/// the permanent that left (`permanentLeftBattlefield`) had a revival counter
fn had_revival(game: &Game, _you: Seat, source: ObjId) -> bool {
    revival(game, source) > 0
}

/// `ReturnSourceFromGraveyardToBattlefieldWithCounterEffect` with one fewer revival counter than it had
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (o, zcc) = cx.source;
    let counters = revival(game, o) - 1;
    if game.obj(o).zone != Zone::Graveyard || game.obj(o).zcc != zcc {
        return Some(());
    }
    cx.run(game, d, &[EffectDef::ReturnSourceFromGraveyardToBattlefieldEffect { tapped: false }]);
    if game.obj(o).zone == Zone::Battlefield && counters > 0 {
        add_counters_by(game, o, "revival", counters, Some(cx.controller()));
    }
    Some(())
}
