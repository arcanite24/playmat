//! `GainActivatedAbilitiesOfTopCardEffect(filter)` (Conspicuous Snoop: "As long as the top card of your library is a
//! Goblin card, Conspicuous Snoop has all activated abilities of that card.")

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("GainActivatedAbilitiesOfTopCardEffect", affects, Layer::Ability(gives)).args(&["filter"])];

/// the source itself, while the top card of its controller's library passes the filter
fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source && top(game, cx).is_some()
}

fn top(game: &Game, cx: &StaticCx) -> Option<ObjId> {
    let card = game.player(cx.controller).library.first().copied()?;
    cx.matches(game, 0, card).then_some(card)
}

/// `ability.isActivatedAbility()` of each of the card's abilities: its activated and mana abilities, those that work
/// on the battlefield (one a card activates from another zone can't be activated from a permanent anyway)
fn gives(game: &Game, cx: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    let Some(card) = top(game, cx) else { return Vec::new() };
    game.obj(card)
        .def
        .abilities
        .iter()
        .filter(|a| {
            let elsewhere = matches!(
                a,
                AbilityDef::CyclingAbility { .. }
                    | AbilityDef::ChannelAbility { .. }
                    | AbilityDef::NinjutsuAbility { .. }
                    | AbilityDef::TurnFaceUpAbility { .. }
                    | AbilityDef::GraveyardActivatedAbility { .. }
            );
            !elsewhere && (a.activated().is_some() || a.mana().is_some())
        })
        .cloned()
        .collect()
}
