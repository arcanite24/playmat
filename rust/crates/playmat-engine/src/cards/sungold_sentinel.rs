//! Sungold Sentinel: "Whenever Sungold Sentinel enters the battlefield or attacks, exile up to one target card from a
//! graveyard. Coven — {1}{W}: Choose a color. Sungold Sentinel gains hexproof from that color until end of turn and
//! can't be blocked by creatures of that color this turn. Activate only if you control three or more creatures with
//! different powers."

use super::*;
use crate::card::{Color, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("SungoldSentinel.SungoldSentinelEffect", apply)];

const NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];

/// `ChoiceColor(true)` ("Choose color"), then, while it's still the permanent the ability came from, until end of
/// turn: hexproof from that color (`HexproofBaseAbility.getFirstFromColor`) and "can't be blocked by creatures of
/// that color" (`CantBeBlockedByAllSourceEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let choices: Vec<String> = NAMES.iter().map(|n| n.to_string()).collect();
    let q = Query::Choice { message: "Choose color".into(), choices };
    let k = loop {
        match ask(game, d, cx.controller(), &q) {
            Answer::Text(t) => {
                if let Some(k) = NAMES.iter().position(|n| *n == t) {
                    break k;
                }
            }
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let (id, zcc) = cx.source;
    let from = NAMES[k].to_lowercase();
    let hexproof = AbilityDef::HexproofFromAbility { rule: format!("hexproof from {from}"), from };
    let filter = Filter(vec![Pred::Type { t: CardType::Creature }, Pred::Color { c: Color::ALL[k] }]);
    for ability in [hexproof, AbilityDef::CantBeBlockedByAbility { filter }] {
        Arc::make_mut(&mut game.grants).push(crate::state::Grant {
            target: id,
            zcc,
            ability,
            until_end_of_turn: true,
            until_turn_of: None,
        });
    }
    Some(())
}
