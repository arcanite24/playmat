//! Ovika, Enigma Goliath: "Whenever you cast a noncreature spell, create X 1/1 red Phyrexian Goblin creature tokens,
//! where X is the mana value of that spell. They gain haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OvikaEnigmaGoliath.OvikaEnigmaGoliathEffect", apply)
    .effects(&["new CreateTokenEffect(new PhyrexianGoblinToken())"])];

/// the cast trigger's X is the spell's mana value (`getValue("spellCast")`); none for 0. The tokens gain haste until
/// end of turn (`GainAbilityTargetEffect` with `FixedTargets(token)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    if x < 1 {
        return Some(());
    }
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { amount, haste_until_end_of_turn, .. } = &mut e {
        *amount = crate::card::Amount::Fixed(x);
        *haste_until_end_of_turn = true;
    }
    cx.run(game, d, &[e]);
    Some(())
}
