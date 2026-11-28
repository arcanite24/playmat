//! `TargetPlayerShufflesTargetCardsEffect` with `TargetCardInTargetPlayersGraveyard(n)`: "Target player shuffles up
//! to N target cards from their graveyard into their library." (Gaea's Blessing, Witness the Future, Quandrix
//! Command)

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TargetPlayerShufflesTargetCardsEffect", apply).args(&["int"]),
    Handler::predicate("TargetCardInTargetPlayersGraveyard", any_graveyard_card),
];

/// any graveyard card: the class overrides only `canTarget(id, source, game)`, which neither choosing the targets
/// (`possibleTargets`, `canTarget(playerId, id, …)`) nor the check as the spell resolves calls, so a card from
/// another player's graveyard can be chosen, and stays legal
fn any_graveyard_card(_game: &Game, _you: Seat, _source: Option<ObjId>, _id: ObjId) -> bool {
    true
}

/// `shuffleCardsToLibrary`: the target cards still there (none: nothing) put into their owners' libraries
/// (`moveCards` to the library: on top, in target order), then the target player's library is shuffled. The player
/// is target `targetPlayerIndex` (0 unless given: Rite of Renewal's 1), the cards the next target.
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let at = cx.arg_int(0).unwrap_or(0).max(0) as usize;
    let Some(p) = cx.player(at) else { return Some(()) };
    let cards = cx.objs(at + 1);
    if cards.is_empty() {
        return Some(());
    }
    for c in cards {
        to_library(game, c, true);
    }
    let mut library = game.player(p).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(p).library = library;
    Some(())
}
