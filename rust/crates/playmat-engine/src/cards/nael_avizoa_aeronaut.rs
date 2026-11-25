//! Nael, Avizoa Aeronaut: "… Then if there are five basic land types among lands you control, draw a card."

use super::*;
use crate::card::{Amount, Value};

pub const HANDLERS: &[Handler] = &[Handler::effect("NaelAvizoaAeronaut.NaelAvizoaAeronautEffect", apply)];

/// `DomainValue.REGULAR` of five: draw a card
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if game.value(&Amount::Of(Value::Domain), you, cx.source_id(), cx.item.x) >= 5 {
        draw(game, you, 1);
    }
    Some(())
}
