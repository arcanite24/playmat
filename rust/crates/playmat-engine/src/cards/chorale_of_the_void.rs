//! Chorale of the Void: "Whenever enchanted creature attacks, put target creature card from defending player's
//! graveyard onto the battlefield under your control tapped and attacking. Void — At the beginning of your end step,
//! sacrifice this Aura unless a nonland permanent left the battlefield this turn or a spell was warped this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("ChoraleOfTheVoid.ChoraleOfTheVoidPredicate", defending_owner)];

/// the card is owned by the defending player of the creature the source is (or, gone, was) attached to
/// (`getSourcePermanentOrLKI`); a creature removed from combat, or gone, still has the player it was attacking
/// (`getDefendingPlayerId` allows former attackers): in a duel, the active player's opponent
fn defending_owner(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let s = game.obj(src);
    let Some((attacker, zcc)) = (if s.zone == Zone::Battlefield { s.attached_to } else { s.lki_attached_to }) else {
        return false;
    };
    let a = game.obj(attacker);
    if a.zone == Zone::Battlefield && a.zcc == zcc {
        if let Some(p) = a.attacking {
            return game.obj(id).owner == p;
        }
    }
    let combat = matches!(
        game.step,
        Some(Step::DeclareAttackers | Step::DeclareBlockers | Step::FirstCombatDamage | Step::CombatDamage | Step::EndCombat)
    );
    combat
        && game.attacked_this_turn.contains(&(attacker, zcc))
        && game.active.is_some_and(|p| game.obj(id).owner == game.opponent(p))
}
