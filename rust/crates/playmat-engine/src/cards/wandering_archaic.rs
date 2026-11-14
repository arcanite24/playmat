//! Wandering Archaic: "Whenever an opponent casts an instant or sorcery spell, they may pay {2}. If they don't, you
//! may copy that spell. You may choose new targets for the copy."

use super::*;
use crate::card::CostDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("WanderingArchaic.WanderingArchaicEffect", apply)];

/// the opponent the trigger points at (`SetTargetPointer.PLAYER`) and the spell (`getValue("spellCast")`): `canPay`
/// (a mana cost always can), "Pay {2}?", the payment; else "Copy <spell>?" and `createCopyOnStack` with new targets
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    let Some(original) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
    else {
        return Some(());
    };
    let cost = [CostDef::ManaCost { cost: "{2}".into() }];
    if pay_unless(game, d, opponent, cx.source_id(), &cost)? {
        return Some(());
    }
    let you = cx.controller();
    let q = Query::YesNo { message: format!("Copy {}?", game.obj(spell).name()) };
    let yes = loop {
        match ask(game, d, you, &q) {
            Answer::Bool(b) => break b,
            Answer::Stop => return None,
            _ => {}
        }
    };
    if yes {
        copy_spell_with(game, d, &original, you, true, false)?;
    }
    Some(())
}
