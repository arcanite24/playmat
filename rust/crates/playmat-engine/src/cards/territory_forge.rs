//! Territory Forge: "When Territory Forge enters, if you cast it, exile target artifact or land. Territory Forge has
//! all activated abilities of the exiled card."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("TerritoryForge.TerritoryForgeStaticEffect", itself, Layer::Ability(gives))];

fn itself(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// the cards in its exile zone for the permanent it is now (`getExileZoneId(sourceId, zcc)`), still in exile
fn exiled(game: &Game, cx: &StaticCx) -> Vec<ObjId> {
    let zcc = game.obj(cx.source).zcc;
    game.exiled_for_source
        .iter()
        .filter(|(s, z, o)| *s == cx.source && *z == zcc && game.obj(*o).zone == Zone::Exile)
        .map(|(_, _, o)| *o)
        .collect()
}

/// `ability.isActivatedAbility()` of each exiled card's abilities: its activated and mana abilities, those that work
/// on the battlefield (one a card activates from another zone can't be activated from a permanent anyway)
fn gives(game: &Game, cx: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    let mut out = Vec::new();
    for card in exiled(game, cx) {
        out.extend(
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
                .cloned(),
        );
    }
    out
}
