//! Karametra's Blessing: "Target creature gets +2/+2 until end of turn. If it's an enchanted creature or enchantment
//! creature, it also gains hexproof and indestructible until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KarametrasBlessing.KarametrasBlessingEffect", apply).effects(&[
    "new GainAbilityTargetEffect(HexproofAbility.getInstance(), Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn)",
])];

/// `EnchantmentOrEnchantedPredicate`: an enchantment, or a permanent with an Aura attached
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let enchanted = game.has_type(o, CardType::Enchantment) || game.attachments(o).any(|a| game.has_subtype(a, "Aura"));
    if enchanted {
        cx.run(game, d, cx.effects);
    }
    Some(())
}
