//! Thought-Stalker Warlock: "Menace. When this creature enters, choose target opponent. If they lost life this turn,
//! they reveal their hand, you choose a nonland card from it, and they discard that card. Otherwise, they discard a
//! card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("ThoughtStalkerWarlock.ThoughtStalkerWarlockCondition", target_lost_life)];

/// `PlayerLostLifeWatcher.getLifeLost(source.getFirstTarget()) > 0`: the trigger's target opponent, read from the
/// ability resolving
fn target_lost_life(game: &Game, _you: Seat, _: ObjId) -> bool {
    let Some(item) = game.resolving_item.as_ref() else { return false };
    let first = item.targets.first().and_then(|ts| ts.first());
    matches!(first, Some(Target::Player(p)) if game.player(*p).life_lost > 0)
}
