//! Harness (the Infinity Stones): `HarnessSourceEffect`, "Harness {this}. (Once harnessed, its ∞ ability is
//! active.)", marks the source permanent harnessed while it stays (`Permanent.setHarnessed`); and
//! `GainHarnessedAbilitySourceEffect(ability)`, "∞ — …": layer 6, the source has the ability while it's harnessed.

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("HarnessSourceEffect", harness),
    Handler::continuous("GainHarnessedAbilitySourceEffect", harnessed_source, Layer::Ability(gives)).args(&["ability"]),
];

const MARK: &str = "harnessed";

/// `source.getSourcePermanentIfItStillExists(game)`, then `setHarnessed(true)`
fn harness(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.source_on_battlefield(game) {
        let o = game.obj_mut(cx.source_id());
        if !o.marks.iter().any(|m| m == MARK) {
            o.marks.push(MARK.into());
        }
    }
    Some(())
}

/// the source itself, while it's harnessed
fn harnessed_source(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source && game.obj(id).marks.iter().any(|m| m == MARK)
}

fn gives(_: &Game, cx: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    cx.abilities.to_vec()
}
