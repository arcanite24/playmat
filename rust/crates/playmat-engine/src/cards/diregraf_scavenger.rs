//! Diregraf Scavenger: "When Diregraf Scavenger enters, exile up to one target card from a graveyard. If a creature
//! card was exiled this way, each opponent loses 2 life and you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DiregrafScavenger.DiregrafScavengerEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0).filter(|c| game.obj(*c).zone == Zone::Graveyard) else { return Some(()) };
    let creature = game.has_type(c, CardType::Creature);
    game.move_to(c, Zone::Exile);
    if creature {
        let you = cx.controller();
        let n = game.seats();
        for i in 1..n {
            let p = (you + i) % n;
            if !game.player(p).lost {
                lose_life(game, p, 2);
            }
        }
        gain_life(game, you, 2);
    }
    Some(())
}
