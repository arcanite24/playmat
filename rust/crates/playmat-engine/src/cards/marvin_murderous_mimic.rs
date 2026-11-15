//! Marvin, Murderous Mimic: "Marvin, Murderous Mimic has all activated abilities of creatures you control that don't
//! have the same name as this creature."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("MarvinMurderousMimic.MarvinMurderousMimicEffect", itself, Layer::Ability(gives))];

/// `source.getSourcePermanentIfItStillExists(game)`: Marvin alone
fn itself(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source && game.obj(id).zone == Zone::Battlefield
}

/// `CardUtil.haveSameNames`: both names nonempty and equal
fn same_names(a: &str, b: &str) -> bool {
    !a.is_empty() && !b.is_empty() && a == b
}

/// the activated (and mana) abilities of each creature you control without Marvin's name, in battlefield order:
/// those it prints and still has, then those it's given
fn gives(game: &Game, cx: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    let me = game.obj(cx.source).name();
    let mut out = Vec::new();
    for p in game.battlefield.iter().copied() {
        if p == cx.source
            || game.obj(p).controller != cx.controller
            || !game.has_type(p, CardType::Creature)
            || same_names(game.obj(p).name(), me)
        {
            continue;
        }
        let printed = game.obj(p).def.abilities.iter().filter(|a| game.has_ability(p, a)).cloned();
        for a in printed.chain(game.granted_abilities(p)) {
            let elsewhere = matches!(
                a,
                AbilityDef::CyclingAbility { .. }
                    | AbilityDef::ChannelAbility { .. }
                    | AbilityDef::NinjutsuAbility { .. }
                    | AbilityDef::TurnFaceUpAbility { .. }
                    | AbilityDef::GraveyardActivatedAbility { .. }
            );
            if !elsewhere && (a.activated().is_some() || a.mana().is_some()) {
                out.push(a);
            }
        }
    }
    out
}
