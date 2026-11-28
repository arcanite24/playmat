//! Urborg Scavengers: "Whenever Urborg Scavengers enters or attacks, exile target card from a graveyard. Put a
//! +1/+1 counter on Urborg Scavengers. Urborg Scavengers has flying as long as a card exiled with it has flying. The
//! same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace,
//! reach, trample, and vigilance."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("UrborgScavengers.UrborgScavengersEffect", itself, Layer::Ability(exiled_keywords))];

fn itself(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// those keywords of the cards in its exile zone (exiled with it as the permanent it is now), each once
fn exiled_keywords(game: &Game, cx: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    let me = (cx.source, game.obj(cx.source).zcc);
    let mut out: Vec<AbilityDef> = Vec::new();
    let exiled = game
        .exiled_for_source
        .iter()
        .filter(|(s, z, c)| (*s, *z) == me && game.obj(*c).zone == Zone::Exile)
        .map(|(_, _, c)| *c);
    for c in exiled {
        for a in &game.obj(c).def.abilities {
            let keep = matches!(
                a,
                AbilityDef::FlyingAbility
                    | AbilityDef::FirstStrikeAbility
                    | AbilityDef::DoubleStrikeAbility
                    | AbilityDef::DeathtouchAbility
                    | AbilityDef::HasteAbility
                    | AbilityDef::HexproofAbility
                    | AbilityDef::HexproofFromAbility { .. }
                    | AbilityDef::IndestructibleAbility
                    | AbilityDef::LifelinkAbility
                    | AbilityDef::MenaceAbility
                    | AbilityDef::ReachAbility
                    | AbilityDef::TrampleAbility
                    | AbilityDef::VigilanceAbility
            );
            if keep && !out.contains(a) {
                out.push(a.clone());
            }
        }
    }
    out
}
