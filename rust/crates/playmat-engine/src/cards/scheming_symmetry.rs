//! Scheming Symmetry: "Choose two target players. Each of them searches their library for a card, then shuffles and
//! puts that card on top."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SchemingSymmetry.SchemingSymmetryEffect", apply)];

/// each target player in order: `searchLibrary` for any card; found, they shuffle and put it on top
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let players: Vec<Seat> = cx
        .legal
        .first()
        .map(|ts| ts.iter().filter_map(|t| if let Target::Player(p) = t { Some(*p) } else { None }).collect())
        .unwrap_or_default();
    for p in players {
        let found = search_library(game, d, p, &Filter(Vec::new()), 1, 1, cx.source_id())?;
        let Some(&card) = found.first() else { continue };
        cx.run_as(game, d, &[EffectDef::ShuffleLibrarySourceEffect], p);
        if d.stopped() {
            return None;
        }
        if !put_on_library(game, d, p, &[card], true, false) {
            return None;
        }
    }
    Some(())
}
