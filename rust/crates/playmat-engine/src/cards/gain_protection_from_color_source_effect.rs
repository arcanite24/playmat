//! `GainProtectionFromColorSourceEffect(Duration.EndOfTurn)`: "{this} gains protection from the color of your choice
//! until end of turn" (Cartel Aristocrat, Pristine Skywise). The color is chosen as the effect starts (`init`), even
//! when the source has left; it's gained only by the permanent the ability came from.

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("GainProtectionFromColorSourceEffect", apply).args(&["one_of:Duration.EndOfTurn"])];

const NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let choices: Vec<String> = NAMES.iter().map(|n| n.to_string()).collect();
    let q = Query::Choice { message: "Choose color for protection ability".into(), choices };
    let color = loop {
        match ask(game, d, cx.controller(), &q) {
            Answer::Text(t) => {
                if let Some(i) = NAMES.iter().position(|n| *n == t) {
                    break Color::ALL[i];
                }
            }
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if cx.source_on_battlefield(game) {
        grant(game, cx.source_id(), &AbilityDef::ProtectionFromColorAbility { c: color });
    }
    Some(())
}
