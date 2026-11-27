//! Galvanic Discharge: "Choose target creature or planeswalker. You get {E}{E}{E}, then you may pay any amount of {E}.
//! Galvanic Discharge deals that much damage to that permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GalvanicDischarge.GalvanicDischargeEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    add_energy(game, you, 3);
    let max = energy(game, you);
    let q = Query::Amount { min: 0, max };
    let paid = loop {
        match ask(game, d, you, &q) {
            Answer::Int(n) if (0..=max).contains(&n) => break n,
            Answer::Stop => return None,
            _ => {}
        }
    };
    if paid <= 0 {
        return Some(());
    }
    add_energy(game, you, -paid);
    if let Some(target) = cx.obj(0) {
        if game.obj(target).zone == Zone::Battlefield {
            deal_damage(game, cx.source_id(), Target::Obj(target), paid, false);
        }
    }
    Some(())
}
