//! Carnage, Crimson Chaos: "When Carnage enters, return target creature card with mana value 3 or less from your
//! graveyard to the battlefield. It gains \"This creature attacks each combat if able\" and \"When this creature
//! deals combat damage to a player, sacrifice it.\""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CarnageCrimsonChaos.CarnageCrimsonChaosReturnEffect", apply).effects(&[
    "new GainAbilityTargetEffect(new AttacksEachCombatStaticAbility(), Duration.WhileOnBattlefield)",
    "new GainAbilityTargetEffect(new DealsCombatDamageToAPlayerTriggeredAbility(new SacrificeSourceEffect()), Duration.WhileOnBattlefield)",
])];

/// The card onto the battlefield; the permanent it became gains both abilities for as long as it stays
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = cx.obj(0)?;
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        cx.run_on(game, d, &cx.effects[..2], vec![Target::Obj(card)]);
    }
    Some(())
}
