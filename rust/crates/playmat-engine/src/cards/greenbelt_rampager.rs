//! Greenbelt Rampager: "When this creature enters, pay {E}{E} (two energy counters). If you can't, return this
//! creature to its owner's hand and you get {E}."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GreenbeltRampager.GreenbeltRampagerEffect", apply)];

/// `PayEnergyCost(2).pay`: two energy paid when you have them (nothing asked); else the creature, still there, back
/// to its owner's hand and one energy
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if energy(game, you) >= 2 {
        add_energy(game, you, -2);
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        game.move_to(cx.source_id(), Zone::Hand);
        add_energy(game, you, 1);
    }
    Some(())
}
