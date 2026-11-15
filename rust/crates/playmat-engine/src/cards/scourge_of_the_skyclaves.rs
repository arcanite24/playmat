//! Scourge of the Skyclaves: "Kicker {4}{B}. When you cast this spell, if it was kicked, each player loses half their
//! life, rounded up. Scourge of the Skyclaves's power and toughness are each equal to 20 minus the highest life total
//! among players."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ScourgeOfTheSkyclaves.ScourgeOfTheSkyclavesEffect", apply),
    Handler::value("ScourgeOfTheSkyclaves.ScourgeOfTheSkyclavesValue", twenty_minus_highest),
];

/// the players still in the game, from `you` in turn order (`getPlayersInRange`)
fn in_range(game: &Game, you: Seat) -> Vec<Seat> {
    let n = game.seats();
    (0..n).map(|i| (you + i) % n).filter(|p| !game.player(*p).lost).collect()
}

/// 20 minus the highest life total among players (0 with none)
fn twenty_minus_highest(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    20 - in_range(game, you).into_iter().map(|p| game.player(p).life).max().unwrap_or(0)
}

/// each player in turn, from you, loses half their life as it is then, rounded up
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for p in in_range(game, cx.controller()) {
        let life = game.player(p).life;
        let n = (life as f64 / 2.0).ceil() as i32;
        if n > 0 {
            lose_life_by(game, p, n, Some(cx.source_id()), false);
        }
    }
    Some(())
}
