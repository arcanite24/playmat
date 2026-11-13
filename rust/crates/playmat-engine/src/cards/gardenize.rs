//! Gardenize: "At the beginning of your first main phase, add {G} for each charge counter on this enchantment."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Gardenize.GardenizeEffect", apply)];

/// With Gardenize on the battlefield (`getPermanent(sourceId)`): {G} for each charge counter on it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield {
        return Some(());
    }
    let n = game.obj(src).counter("charge");
    if n > 0 {
        cx.run(game, d, &[EffectDef::BasicManaEffect { colored: [0, 0, 0, 0, n as u32], colorless: 0, until_end_of_turn: false }]);
    }
    Some(())
}
