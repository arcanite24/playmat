//! Security Bypass: "As long as enchanted creature is attacking alone, it can't be blocked."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SecurityBypass.SecurityBypassCondition", attacking_alone)];

/// one attacker in combat (`Combat.attacksAlone`), and it's the creature the Aura is attached to
fn attacking_alone(game: &Game, _: Seat, source: ObjId) -> bool {
    if game.obj(source).zone != Zone::Battlefield {
        return false;
    }
    let Some(to) = game.attached(source) else { return false };
    let all = attackers(game);
    all.len() == 1 && all[0] == to
}
