//! Ursine Champion: "{5}{G}: Ursine Champion gets +3/+3 and becomes a Bear Berserker until end of turn. Activate
//! only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UrsineChampion.UrsineChampionEffect", apply)];

/// Layer 4 until end of turn: the source permanent's creature types become Bear Berserker (`removeAllCreatureTypes`,
/// then `addSubType`), its other subtypes kept; the cleanup gives it its own back, as the engine's other "until end of
/// turn" type changes do
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield {
        return Some(());
    }
    let old = game.obj(src).def.clone();
    let mut def = (*old).clone();
    def.subtypes.retain(|s| crate::card::NON_CREATURE_TYPES.contains(&s.as_str()));
    def.all_creature_types = false;
    for s in ["Bear", "Berserker"] {
        if !def.subtypes.iter().any(|x| x == s) {
            def.subtypes.push(s.to_string());
        }
    }
    let o = game.obj_mut(src);
    if o.until_end_of_turn_def.is_none() {
        o.until_end_of_turn_def = Some(old);
    }
    o.def = std::sync::Arc::new(def);
    Some(())
}
