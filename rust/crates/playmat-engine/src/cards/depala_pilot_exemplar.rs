//! Depala, Pilot Exemplar: "Other Dwarves you control get +1/+1. Each Vehicle you control gets +1/+1 as long as it's
//! a creature. Whenever Depala becomes tapped, you may pay {X}. If you do, reveal the top X cards of your library, put
//! all Dwarf and Vehicle cards from among them into your hand, then put the rest on the bottom of your library in a
//! random order."

use super::*;
use crate::card::{Amount, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::effect("DepalaPilotExemplar.DepalaPilotExemplarEffect", apply).filters(&["filter"])];

/// (the trigger is optional, so it asked already) `announceX(0, MAX)`, then {X} paid; paid with X above 0:
/// `RevealLibraryPutIntoHandEffect(X, Dwarf or Vehicle cards, Zone.LIBRARY, false)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: i32::MAX }) {
            Answer::Int(n) if n >= 0 => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if x <= 0 {
        return Some(());
    }
    let cost = ManaCost { generic: x as u32, ..ManaCost::default() };
    let before = game.clone();
    if !pay_mana(game, d, you, &cost) {
        if d.stopped() {
            return None;
        }
        *game = before;
        return Some(());
    }
    let e = EffectDef::RevealLibraryPutIntoHandEffect {
        amount: Amount::Fixed(x),
        filter: cx.filters[0].clone(),
        graveyard: false,
        any_order: false,
    };
    cx.run(game, d, &[e]);
    Some(())
}
