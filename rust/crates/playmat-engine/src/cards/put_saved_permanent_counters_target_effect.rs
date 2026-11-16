//! `PutSavedPermanentCountersTargetEffect(key)`: "if it had counters on it, put those counters on up to one target
//! creature" (Reluctant Role Model's "creatureDied", the creature that died; Iron Apprentice and Scolding
//! Administrator's "permanentLeftBattlefield", the source itself): the counters it had as it left (its last known
//! information), each kind put on the target in the order Java's `Counters` (a `HashMap`) walks them.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PutSavedPermanentCountersTargetEffect", apply)
    .args(&["one_of:\"creatureDied\"|\"permanentLeftBattlefield\""])];

/// The bucket a key falls in, in a `HashMap` of the default capacity (16; the counter kinds of one permanent stay
/// under its 12-entry threshold): `String.hashCode`, spread by `HashMap.hash`
fn bucket(name: &str) -> u32 {
    let h = name.encode_utf16().fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(c as i32)) as u32;
    (h ^ (h >> 16)) & 15
}

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let from = if cx.args.first().is_some_and(|a| a.contains("creatureDied")) { cx.item.died?.0 } else { cx.source_id() };
    let to = cx.obj(0)?;
    if game.obj(to).zone != crate::state::Zone::Battlefield {
        return Some(());
    }
    let mut counters: Vec<(String, i32)> = game.obj(from).lki_counters.iter().filter(|(_, n)| *n > 0).cloned().collect();
    // a stable sort: within a bucket, the order the kinds were first added
    counters.sort_by_key(|(name, _)| bucket(name));
    for (name, n) in counters {
        put_counters(game, to, &name, n, cx.controller());
    }
    Some(())
}
