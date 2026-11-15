//! Council of the Absolute: "Your opponents can't cast spells with the chosen name." and "Spells with the chosen name
//! you cast cost {2} less to cast."

use super::*;
use crate::card::{CardDef, ManaCost};

pub const HANDLERS: &[Handler] = &[
    Handler::rule("CouncilOfTheAbsolute.CouncilOfTheAbsoluteReplacementEffect", Rule::CantCast(cant_cast)),
    Handler::cost(
        "CouncilOfTheAbsolute.CouncilOfTheAbsoluteCostReductionEffect",
        CostKind::Reduce,
        your_named_spell,
        two_less,
    ),
];

/// the name `ChooseACardNameEffect` kept on it
fn chosen(game: &Game, cx: &StaticCx) -> Option<String> {
    game.obj(cx.source).chosen_name.clone().filter(|n| !n.is_empty())
}

/// an opponent casting a spell with the chosen name
fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, _: ObjId, face: &CardDef) -> bool {
    cx.opponent(seat) && chosen(game, cx).is_some_and(|n| face.name == n)
}

/// a `SpellAbility` you control of a card with the chosen name (`CardUtil.haveSameNames`)
fn your_named_spell(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none()
        && of.seat == cx.controller
        && chosen(game, cx).is_some_and(|n| game.obj(of.card).name() == n || of.face.name == n)
}

/// `CardUtil.reduceCost(ability, 2)`
fn two_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(2);
}
