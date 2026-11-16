//! Awaken the Sleeper: "Gain control of target creature until end of turn. Untap that creature. It gains haste until
//! end of turn. If it's equipped, you may destroy all Equipment attached to that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AwakenTheSleeper.AwakenTheSleeperEffect", apply)];

/// equipped (`EquippedPredicate`): "Destroy all equipment attached to <name>?", then each Equipment attached is
/// destroyed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(creature).zcc;
    let equipment: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|e| game.obj(*e).attached_to == Some((creature, zcc)) && game.has_subtype(*e, "Equipment"))
        .collect();
    if equipment.is_empty() {
        return Some(());
    }
    let message = format!("Destroy all equipment attached to {}?", game.obj(creature).name());
    if !choose_use(game, d, cx.controller(), &message)? {
        return Some(());
    }
    for e in equipment {
        destroy(game, e);
    }
    Some(())
}
