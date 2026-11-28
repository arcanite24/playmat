//! `ExchangeLifeTwoTargetEffect`: "two target players exchange life totals" (Axis of Mortality): the first target
//! player's `exchangeLife` with the second, each `setLife` a gain or a loss, the first's first.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExchangeLifeTwoTargetEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let players: Vec<Seat> = cx
        .legal
        .first()?
        .iter()
        .filter_map(|t| if let Target::Player(p) = t { Some(*p) } else { None })
        .collect();
    let (&p1, &p2) = (players.first()?, players.get(1)?);
    let (l1, l2) = (game.player(p1).life, game.player(p2).life);
    if l1 != l2 {
        set_life(game, p1, l2);
        set_life(game, p2, l1);
    }
    Some(())
}

/// `PlayerImpl.setLife`: a gain or a loss of the difference
fn set_life(game: &mut Game, p: Seat, to: i32) {
    let life = game.player(p).life;
    if to > life {
        gain_life(game, p, to - life);
    } else if to < life {
        lose_life(game, p, life - to);
    }
}
