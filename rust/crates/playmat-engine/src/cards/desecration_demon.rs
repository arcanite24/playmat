//! Desecration Demon: "At the beginning of each combat, any opponent may sacrifice a creature. If a player does,
//! tap Desecration Demon and put a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DesecrationDemon.DesecrationDemonEffect", apply)];

/// While the Demon is on the battlefield, each opponent who controls a creature is asked; a yes, then the creature
/// chosen (`TargetControlledPermanent(1, 1)` with `Player.choose`) is sacrificed, and the Demon taps and gets the
/// counter
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let demon = cx.source_id();
    if game.obj(demon).zone != Zone::Battlefield {
        return Some(());
    }
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let creatures = controlled_creatures(game, p);
        if creatures.is_empty() {
            continue;
        }
        let message = format!("Sacrifice a creature to tap {} and put a +1/+1 counter on it?", game.obj(demon).name());
        if !choose_use(game, d, p, &message)? {
            continue;
        }
        let chosen = choose_cards(game, d, p, &creatures, &creatures, 1, 1, "sacrifice")?;
        if let Some(&c) = chosen.first() {
            if game.obj(c).zone == Zone::Battlefield {
                sacrifice(game, c);
                tap(game, demon);
                put_counters(game, demon, "+1/+1", 1, you);
            }
        }
    }
    Some(())
}
