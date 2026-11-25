//! `LoseAllCreatureTypesTargetEffect(Duration.EndOfTurn)`: "target creature … loses all creature types until end of
//! turn" (Nameless Inversion).

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("LoseAllCreatureTypesTargetEffect", apply).args(&["one_of:Duration.EndOfTurn"])];

/// Layer 4 `removeAllCreatureTypes` on the target until end of turn, as the engine keeps an end-of-turn change of a
/// permanent's card: its creature subtypes (and "every creature type") gone, its card as it was back at cleanup
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let old = game.obj(o).def.clone();
    let mut def = (*old).clone();
    def.subtypes.retain(|s| crate::card::NON_CREATURE_TYPES.contains(&s.as_str()));
    def.all_creature_types = false;
    let ob = game.obj_mut(o);
    if ob.until_end_of_turn_def.is_none() {
        ob.until_end_of_turn_def = Some(old);
    }
    ob.def = std::sync::Arc::new(def);
    Some(())
}
