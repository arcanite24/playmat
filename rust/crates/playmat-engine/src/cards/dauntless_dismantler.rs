//! Dauntless Dismantler: "{X}{X}{W}, Sacrifice Dauntless Dismantler: Destroy each artifact with mana value X."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DauntlessDismantler.DauntlessDismantlerEffect", apply)];

/// each artifact with mana value X, in battlefield order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x.max(0) as u32;
    let all: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Artifact) && game.obj(*o).def.mana_value() == x)
        .collect();
    for o in all {
        destroy(game, o);
    }
    Some(())
}
