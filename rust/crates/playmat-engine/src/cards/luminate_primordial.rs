//! Luminate Primordial: "Vigilance. When Luminate Primordial enters, for each opponent, exile up to one target
//! creature that player controls and that player gains life equal to its power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LuminatePrimordial.LuminatePrimordialEffect", apply)];

/// Each target's power and controller as it is, then `moveCards` of them all to exile, then each of those players
/// gains the life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut targets: Vec<ObjId> = Vec::new();
    for i in 0..cx.legal.len() {
        for o in cx.objs(i) {
            if game.obj(o).zone == Zone::Battlefield && !targets.contains(&o) {
                targets.push(o);
            }
        }
    }
    let gains: Vec<(Seat, i32)> =
        targets.iter().map(|o| (game.obj(*o).controller, game.power_toughness(*o).0)).collect();
    if !targets.is_empty() && !put_cards(game, d, you, &targets, crate::card::PutCards::Exiled) {
        return None;
    }
    for (p, n) in gains {
        if n > 0 && !game.player(p).lost {
            gain_life(game, p, n);
        }
    }
    Some(())
}
