//! Alacrian Armory: "At the beginning of combat on your turn, choose up to one target Mount or Vehicle you control.
//! Until end of turn, that permanent becomes saddled if it's a Mount and becomes an artifact creature if it's a
//! Vehicle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AlacrianArmory.AlacrianArmoryAnimateEffect", apply)];

/// A Mount is saddled as its saddle ability's `SaddleEventEffect` makes it (`SaddleAbility.applySaddle`: only one
/// with saddle); a Vehicle becomes an artifact creature until end of turn (`AddCardTypeTargetEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    if game.has_subtype(target, "Mount") {
        let saddle = game.obj(target).def.abilities.iter().any(|a| {
            matches!(a, AbilityDef::ActivateAsSorceryActivatedAbility { effects, .. }
                if effects.iter().any(|e| matches!(e, EffectDef::SaddleEventEffect)))
        });
        if saddle {
            let mark = format!("saddled{}", game.turn);
            let ob = game.obj_mut(target);
            if !ob.marks.contains(&mark) {
                ob.marks.push(mark);
            }
        }
    }
    if game.has_subtype(target, "Vehicle") {
        let animate = EffectDef::AddCardTypeTargetEffect {
            types: vec![CardType::Creature, CardType::Artifact],
            duration: crate::card::Duration::EndOfTurn,
        };
        cx.run_on(game, d, &[animate], vec![Target::Obj(target)]);
    }
    Some(())
}
