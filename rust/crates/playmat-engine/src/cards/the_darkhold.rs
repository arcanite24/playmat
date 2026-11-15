//! The Darkhold: "{T}: Put a sin counter on The Darkhold. Draw a card for each sin counter on it, then you lose 1
//! life for each card in your hand. {6}{B}{B}, {T}: Each player's life total becomes the lowest life total among all
//! players. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheDarkhold.TheDarkholdSetLifeTotalsEffect", apply)];

/// the lowest life total among the players in range, then each one's `setLife` to it (a gain or a loss), you first
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let players: Vec<Seat> =
        (0..game.seats()).map(|k| (you + k) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    let lowest = players.iter().map(|p| game.player(*p).life).min().unwrap_or(0);
    for p in players {
        let life = game.player(p).life;
        if lowest > life {
            gain_life(game, p, lowest - life);
        } else if lowest < life {
            lose_life(game, p, life - lowest);
        }
    }
    Some(())
}
