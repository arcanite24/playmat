//! `SearchLibraryPutInPlayTargetPlayerEffect(target, tapped)` (Fertilid…): "target player searches their library
//! for a basic land card and puts it onto the battlefield tapped. Then that player shuffles."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SearchLibraryPutInPlayTargetPlayerEffect", apply).args(&["library_target", "text"])];

/// Each target player searches, puts what they found onto the battlefield (tapped, by the second argument), and
/// shuffles
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let bounds: Vec<usize> =
        cx.args.first().map(|a| a.split(' ').filter_map(|n| n.parse().ok()).collect()).unwrap_or_default();
    let (min, max) = match bounds[..] {
        [lo, hi] => (lo, hi),
        _ => (1, 1),
    };
    let to = if cx.args.get(1).map(|a| a.trim()) == Some("true") {
        crate::card::PutCards::BattlefieldTapped
    } else {
        crate::card::PutCards::Battlefield
    };
    let filter = cx.filters.last().cloned().unwrap_or_default();
    let players: Vec<Seat> = cx
        .legal
        .first()
        .map(|ts| ts.iter().filter_map(|t| if let Target::Player(p) = t { Some(*p) } else { None }).collect())
        .unwrap_or_default();
    for p in players {
        let found = search_library(game, d, p, &filter, min, max, cx.source_id())?;
        if !found.is_empty() && !put_cards(game, d, p, &found, to) {
            return None;
        }
        let mut library = game.player(p).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(p).library = library;
    }
    Some(())
}
