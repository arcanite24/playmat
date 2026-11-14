//! The conditions and values of Java's other per-turn watchers (`PermanentsSacrificedWatcher`,
//! `CardsLeftGraveyardWatcher`, `DiscardedCardWatcher`, `ScryOrSurveilWatcher`, `PlayerLostLifeWatcher`,
//! `AttackedThisTurnWatcher`, `CommittedCrimeWatcher`, `CreaturesDiedWatcher`, `ArtifactEnteredControllerWatcher`,
//! `CreatureEnteredControllerWatcher`, `PermanentsEnteredBattlefieldWatcher`, `CardsExiledThisTurnWatcher`), as `Game::watch` keeps them.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("SacrificedArtifactThisTurnCondition", sacrificed_artifact),
    Handler::condition("CardLeftYourGraveyardThisTurnCondition", left_your_graveyard),
    Handler::condition("ControllerDiscardedThisTurnCondition", you_discarded),
    Handler::condition("PlayerDiscardedThisTurnCondition", a_player_discarded),
    Handler::condition("ScryOrSurveilCondition", scried_or_surveilled),
    Handler::condition("LiveLostLastTurnCondition", lost_life_last_turn),
    Handler::condition("CommittedCrimeCondition", committed_crime),
    Handler::condition("ArtifactEnteredUnderYourControlCondition", artifact_entered),
    Handler::condition("CreatureEnteredUnderYourControlCondition", creature_entered),
    Handler::condition("CreatureDiedControlledCondition", your_creature_died),
    Handler::value("CreaturesYouControlDiedCount", your_creatures_died),
    Handler::condition("WasCardExiledThisTurnCondition", card_exiled),
    Handler::value("HobgoblinBanditLord.GoblinsEnteredThisTurnDynamicValue", goblins_entered),
    Handler::value("CreaturesThatAttackedThisTurnCount", attacked_this_turn),
];

fn sacrificed_artifact(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.sacrificed.iter().any(|(s, artifact)| *s == you && *artifact)
}

fn left_your_graveyard(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.left_graveyard.contains(&you)
}

fn you_discarded(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.discarded.contains(&you)
}

/// a player in range discarded a card this turn (a duel: either)
fn a_player_discarded(game: &Game, _: Seat, _: ObjId) -> bool {
    !game.watch.discarded.is_empty()
}

fn scried_or_surveilled(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.scried.contains(&you)
}

fn lost_life_last_turn(game: &Game, you: Seat, _: ObjId) -> bool {
    game.player(you).life_lost_last_turn > 0
}

/// the creatures that attacked this turn, any player's
fn attacked_this_turn(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    game.attacked_this_turn.len() as i32
}

/// you've committed a crime this turn
fn committed_crime(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.crimes.contains(&you)
}

fn artifact_entered(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.entered.iter().any(|(c, artifact, _, _)| *c == you && *artifact)
}

fn creature_entered(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.entered.iter().any(|(c, _, creature, _)| *c == you && *creature)
}

/// a creature died under your control this turn
fn your_creature_died(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.died.contains(&you)
}

fn your_creatures_died(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.died.iter().filter(|c| **c == you).count() as i32
}

/// a card (or token) was put into exile this turn
fn card_exiled(game: &Game, _: Seat, _: ObjId) -> bool {
    game.watch.exiled > 0
}

/// Hobgoblin Bandit Lord: the Goblins that entered under your control this turn
fn goblins_entered(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.entered.iter().filter(|(c, _, _, goblin)| *c == you && *goblin).count() as i32
}
