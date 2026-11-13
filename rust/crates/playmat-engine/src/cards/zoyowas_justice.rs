//! Zoyowa's Justice: "The owner of target artifact or creature with mana value 1 or greater shuffles it into their
//! library. Then that player discovers X, where X is its mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZoyowasJustice.ZoyowasJusticeEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let value = game.obj(o).def.mana_value() as i32;
    let owner = game.obj(o).owner;
    cx.run_on(game, d, &[EffectDef::ShuffleIntoLibraryTargetEffect { optional: false }], vec![Target::Obj(o)]);
    if !discover(game, d, owner, value) {
        return None;
    }
    Some(())
}
