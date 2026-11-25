//! Mordenkainen's Polymorph: "Until end of turn, target creature becomes a Dragon with base power and toughness 4/4
//! and gains flying."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("MordenkainensPolymorph.MordenkainensPolymorphEffect", apply)];

/// Until end of turn (as the engine's end-of-turn `BecomesCreatureTargetEffect` keeps it: the card as it was comes
/// back at cleanup, the base power and toughness a layer 7b effect with its own timestamp): layer 4
/// `removeAllCreatureTypes` and a Dragon (its card types and other subtypes stay), layer 6 flying, layer 7b 4/4
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let old = game.obj(o).def.clone();
    let mut def = (*old).clone();
    def.subtypes.retain(|s| crate::card::NON_CREATURE_TYPES.contains(&s.as_str()));
    def.all_creature_types = false;
    def.subtypes.push("Dragon".into());
    // (`addAbility` of the one `FlyingAbility`: not again on a creature that has it)
    if !def.abilities.contains(&AbilityDef::FlyingAbility) {
        def.abilities.push(AbilityDef::FlyingAbility);
    }
    def.remove_pt_cda();
    def.power = 4;
    def.toughness = 4;
    let (zcc, at) = (game.obj(o).zcc, game.timestamp());
    game.base_pt_until_eot.push((o, zcc, 4, 4, at));
    let ob = game.obj_mut(o);
    if ob.until_end_of_turn_def.is_none() {
        ob.until_end_of_turn_def = Some(old);
    }
    ob.def = std::sync::Arc::new(def);
    Some(())
}
