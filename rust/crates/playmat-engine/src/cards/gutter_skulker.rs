//! Gutter Skulker // Gutter Shortcut: "Enchanted creature can't be blocked as long as it's attacking alone."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("GutterSkulker.GutterShortcutCondition", attacking_alone)];

/// the Aura's creature is the one creature attacking (`Combat.attacksAlone`)
fn attacking_alone(game: &Game, _: Seat, source: ObjId) -> bool {
    if game.obj(source).zone != Zone::Battlefield {
        return false;
    }
    let Some(creature) = game.attached(source) else { return false };
    let attackers: Vec<ObjId> = game.battlefield.iter().copied().filter(|b| game.obj(*b).attacking.is_some()).collect();
    attackers == [creature]
}
