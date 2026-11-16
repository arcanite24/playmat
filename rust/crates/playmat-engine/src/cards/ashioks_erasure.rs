//! Ashiok's Erasure: "When Ashiok's Erasure enters, exile target spell." and "Your opponents can't cast spells with
//! the same name as the exiled card."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AshioksErasure.AshioksErasureExileEffect", exile),
    Handler::rule("AshioksErasure.AshioksErasureReplacementEffect", Rule::CantCast(cant_cast)),
];

/// The target spell exiled into this object's exile zone (its zone-change count as the trigger went on the stack,
/// `getStackMomentSourceZCC`)
fn exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = cx.obj(0)?;
    if game.obj(spell).zone != Zone::Stack {
        return Some(());
    }
    cx.run(game, d, &[EffectDef::ExileTargetEffect]);
    if game.obj(spell).zone == Zone::Exile {
        game.obj_mut(spell).exiled_with = Some(cx.source);
    }
    Some(())
}

/// `CAST_SPELL_LATE` by an opponent of a spell (its characteristics as cast) with the name of the first card in this
/// permanent's exile zone
fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, _: ObjId, face: &CardDef) -> bool {
    if !cx.opponent(seat) || game.obj(cx.source).zone != Zone::Battlefield {
        return false;
    }
    let zone = (cx.source, game.obj(cx.source).zcc);
    game.exile
        .iter()
        .copied()
        .find(|c| game.obj(*c).exiled_with == Some(zone))
        .is_some_and(|c| game.obj(c).name() == face.name)
}
