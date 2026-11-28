//! Pox Plague: "Each player loses half their life, then discards half the cards in their hand, then sacrifices half
//! the permanents they control of their choice. Round down each time."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PoxPlague.PoxPlagueEffect", apply)];

/// Three rounds over the players in range from you: life, then discards (`discard(n)`), then each player chooses
/// and sacrifices half their permanents (`TargetSacrifice(n)`) before the next one chooses
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let players: Vec<Seat> =
        (0..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    for p in players.iter().copied() {
        let n = game.player(p).life.div_euclid(2);
        if n > 0 {
            lose_life_by(game, p, n, Some(cx.source_id()), false);
        }
    }
    for p in players.iter().copied() {
        let n = game.player(p).hand.len() / 2;
        if n > 0 {
            discard(game, d, p, n as i32);
            if d.stopped() {
                return None;
            }
        }
    }
    for p in players.iter().copied() {
        let mine: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == p).collect();
        let n = mine.len() / 2;
        if n == 0 {
            continue;
        }
        let chosen = choose_cards(game, d, p, &mine, &mine, n, n, "sacrifice")?;
        for o in chosen {
            if game.obj(o).zone == Zone::Battlefield {
                sacrifice(game, o);
            }
        }
    }
    Some(())
}
