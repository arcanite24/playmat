//! No Witnesses: "Each player who controls the most creatures investigates. Then destroy all creatures."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("NoWitnesses.NoWitnessesEffect", apply).effects(&["new InvestigateEffect()"])];

/// the players with the most creatures, in turn order from you, each investigate once
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let count = |game: &Game, p: Seat| {
        game.battlefield.iter().filter(|o| game.obj(**o).controller == p && game.has_type(**o, CardType::Creature)).count()
    };
    let n = game.seats();
    let players: Vec<Seat> = (0..n).map(|i| (you + i) % n).filter(|p| !game.player(*p).lost).collect();
    let most = players.iter().map(|p| count(game, *p)).max().unwrap_or(0);
    if most == 0 {
        return Some(());
    }
    for p in players {
        if count(game, p) >= most {
            cx.run_as(game, d, cx.effects, p);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
