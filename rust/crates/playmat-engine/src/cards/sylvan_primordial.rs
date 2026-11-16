//! Sylvan Primordial: "Reach. When Sylvan Primordial enters, for each opponent, destroy target noncreature permanent
//! that player controls. Search your library for up to X Forest cards, where X is the number of permanents destroyed
//! this way, put those cards onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SylvanPrimordial.SylvanPrimordialEffect", apply)
    .effects(&["new SearchLibraryPutInPlayEffect(new TargetCardInLibrary(1, filterForest), true)"])];

/// each target group's permanent (one per opponent, `ForEachPlayerTargetsAdjuster`) destroyed in turn, counting the
/// ones that left; then, with any destroyed, `SearchLibraryPutInPlayEffect(TargetCardInLibrary(X, …), true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut destroyed = 0;
    for i in 0..cx.legal.len() {
        let Some(o) = cx.obj(i) else { continue };
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        destroy(game, o);
        if game.obj(o).zone != Zone::Battlefield {
            destroyed += 1;
        }
    }
    if destroyed == 0 {
        return Some(());
    }
    let mut e = cx.effects[0].clone();
    if let EffectDef::SearchLibraryEffect { max, min, .. } = &mut e {
        *max = destroyed;
        *min = destroyed;
    }
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
