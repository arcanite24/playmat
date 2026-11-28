//! `GainAbilityAllOfChosenSubtypeEffect(ability, WhileOnBattlefield, filter)`: "creatures you control of the chosen
//! type have first strike" (Chronicle of Victory, Radiant Destiny). Layer 6, the filter looked at each time, and of
//! the creature type the source chose as it entered (none chosen: nothing).

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("GainAbilityAllOfChosenSubtypeEffect", affects, Layer::Ability(gives)).args(&[
        "ability",
        "while_on_battlefield",
        "filter",
    ])];

fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    cx.matches(game, 0, id) && game.obj(cx.source).chosen_type.as_ref().is_some_and(|t| game.has_subtype(id, t))
}

fn gives(_: &Game, cx: &StaticCx, _: ObjId) -> Vec<crate::card::AbilityDef> {
    cx.abilities.to_vec()
}
