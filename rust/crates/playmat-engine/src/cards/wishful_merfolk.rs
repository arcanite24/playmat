//! Wishful Merfolk: "Defender. {1}{U}: Wishful Merfolk loses defender and becomes a Human until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WishfulMerfolk.WishfulMerfolkEffect", becomes_human)];

/// Until end of turn (as `LoseAllAbilitiesTargetEffect` does it: the card as it was comes back in the cleanup):
/// layer 4 `removeAllCreatureTypes` and Human, layer 6 defender removed
fn becomes_human(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let ob = game.obj_mut(cx.source_id());
    let old = ob.def.clone();
    let mut def = (*old).clone();
    def.subtypes.retain(|s| crate::card::NON_CREATURE_TYPES.contains(&s.as_str()));
    def.subtypes.push("Human".to_string());
    def.all_creature_types = false;
    def.abilities.retain(|a| !matches!(a, AbilityDef::DefenderAbility));
    ob.def = Arc::new(def);
    ob.creature_types = None;
    if ob.until_end_of_turn_def.is_none() {
        ob.until_end_of_turn_def = Some(old);
    }
    Some(())
}
