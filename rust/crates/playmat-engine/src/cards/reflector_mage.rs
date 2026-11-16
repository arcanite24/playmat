//! Reflector Mage: "When Reflector Mage enters the battlefield, return target creature an opponent controls to its
//! owner's hand. That creature's owner can't cast spells with the same name as that creature until your next turn."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ReflectorMage.ReflectorMageEffect", apply),
    // made by the effect with the creature's name and its owner (`args`: the name, the owner's seat)
    Handler::rule("ReflectorMage.ExclusionRitualReplacementEffect", Rule::CantCast(same_name)).until_your_next_turn(),
];

/// the target (still a permanent) to its owner's hand, then, with a name, the rule-modifying effect until your next
/// turn for its name as a permanent and its owner
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    if game.obj(target).zone != Zone::Battlefield {
        return Some(());
    }
    let name = game.obj(target).name().to_string();
    let owner = game.obj(target).owner;
    if !put_cards(game, d, cx.controller(), &[target], crate::card::PutCards::Hand) {
        return None;
    }
    if name.is_empty() {
        return Some(());
    }
    let handler = serde_json::from_value(serde_json::json!("ReflectorMage.ExclusionRitualReplacementEffect")).ok()?;
    let e = EffectDef::CardStatic {
        handler,
        args: vec![name, owner.to_string()],
        effects: Vec::new(),
        filters: Vec::new(),
        abilities: Vec::new(),
    };
    cx.run_on(game, d, &[e], Vec::new());
    Some(())
}

/// `CAST_SPELL_LATE` of a spell with that name whose card that player owns
fn same_name(game: &Game, cx: &StaticCx, _: Seat, id: ObjId, face: &CardDef) -> bool {
    let (Some(name), Some(owner)) = (cx.args.first(), cx.args.get(1)) else { return false };
    face.name == *name && game.obj(id).owner.to_string() == *owner
}
