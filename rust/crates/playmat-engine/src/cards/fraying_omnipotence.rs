//! Fraying Omnipotence: "Each player loses half their life, then discards half the cards in their hand, then
//! sacrifices half the creatures they control. Round up each time."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FrayingOmnipotence.FrayingOmnipotenceEffect", apply)];

/// three passes over the players in range from you: the life, then the discards (each chooses), then the sacrifices
/// (each chooses its `TargetSacrifice` and sacrifices them before the next player chooses)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let players = |game: &Game| -> Vec<Seat> {
        (0..game.seats()).map(|k| (you + k) % game.seats()).filter(|p| !game.player(*p).lost).collect()
    };
    for p in players(game) {
        let life = game.player(p).life;
        // `Math.ceil(life / 2.0)`; nothing to lose at 0 or less
        let n = (life + 1).div_euclid(2);
        if n > 0 {
            lose_life(game, p, n);
        }
    }
    for p in players(game) {
        let n = game.player(p).hand.len().div_ceil(2);
        if n > 0 {
            discard(game, d, p, n as i32);
            if d.stopped() {
                return None;
            }
        }
    }
    for p in players(game) {
        let creatures: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
            .collect();
        let n = creatures.len().div_ceil(2);
        if n == 0 {
            continue;
        }
        let chosen = choose_cards(game, d, p, &creatures, &creatures, n, n, "sacrifice")?;
        for o in chosen {
            if game.obj(o).zone == Zone::Battlefield {
                sacrifice(game, o);
            }
        }
    }
    Some(())
}
