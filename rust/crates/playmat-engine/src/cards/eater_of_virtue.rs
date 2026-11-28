//! Eater of Virtue: "Whenever equipped creature dies, exile it. Equipped creature gets +2/+0. As long as a card
//! exiled with Eater of Virtue has flying, equipped creature has flying. The same is true for first strike, double
//! strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, and vigilance.
//! Equip {1}"

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("EaterOfVirtue.EaterOfVirtueExileEffect", exile),
    Handler::continuous("EaterOfVirtue.EaterOfVirtueGainAbilityAttachedEffect", equipped, Layer::Ability(exiled_keywords)),
];

/// Java's exile zone is keyed by the Eater's id alone (`getExileZoneId(sourceId + "cards exiled by Eater of
/// Virtue")`), whatever its zone changes since: this stands for "any zone-change count" (not `MAX`, so a `z + 1`
/// elsewhere can't overflow)
const ANY_ZCC: u32 = u32::MAX - 1;

/// With the Eater on the battlefield, the card that died (`SetTargetPointer.CARD`, still that card) is exiled into
/// its zone
fn exile(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let eater = cx.source_id();
    if game.obj(eater).zone != Zone::Battlefield {
        return Some(());
    }
    let Some((Target::Obj(card), zcc)) = cx.item.targets.first().zip(cx.item.target_zcc.first()).and_then(|(ts, zs)| {
        ts.first().copied().zip(zs.first().copied())
    }) else {
        return Some(());
    };
    if game.obj(card).zcc != zcc || game.obj(card).zone == Zone::Exile {
        return Some(());
    }
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        game.obj_mut(card).exiled_with = Some((eater, ANY_ZCC));
    }
    Some(())
}

/// the creature the Eater is attached to
fn equipped(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id)
}

/// those keywords among the abilities of the cards in its exile zone (`card.getAbilities()`), each once
fn exiled_keywords(game: &Game, cx: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    let mut out: Vec<AbilityDef> = Vec::new();
    let exiled = game.exile.iter().copied().filter(|c| game.obj(*c).exiled_with == Some((cx.source, ANY_ZCC)));
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
                    | AbilityDef::IndestructibleAbility
                    | AbilityDef::LifelinkAbility
                    | AbilityDef::MenaceAbility
                    | AbilityDef::ProtectionAbility { .. }
                    | AbilityDef::ProtectionFromColorAbility { .. }
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
