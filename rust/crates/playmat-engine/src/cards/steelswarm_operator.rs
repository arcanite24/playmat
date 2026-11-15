//! Steelswarm Operator: "{T}: Add {U}. Spend this mana only to cast an artifact spell." and "{T}: Add {U}{U}. Spend
//! this mana only to activate abilities of artifact sources."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[
    Handler::spend("SteelswarmOperator.SteelswarmOperatorSpellManaBuilder", spell),
    Handler::spend("SteelswarmOperator.SteelswarmOperatorAbilitiesManaBuilder", abilities),
];

fn spell(game: &Game, cx: &SpendCx) -> bool {
    cx.spell() && game.has_type(cx.on, CardType::Artifact)
}

fn abilities(game: &Game, cx: &SpendCx) -> bool {
    cx.ability && game.has_type(cx.on, CardType::Artifact)
}
