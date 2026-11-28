//! Blitzball: "{T}: Add one mana of any color. GOOOOAAAALLL! — {T}, Sacrifice Blitzball: Draw two cards. Activate
//! only if an opponent was dealt combat damage by a legendary creature this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("Blitzball.BlitzballCondition", legendary_damage)];

/// `BlitzballWatcher`: combat damage to an opponent from a legendary creature (as the source is now)
fn legendary_damage(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.damage.iter().any(|d| {
        d.combat
            && matches!(d.to, Target::Player(p) if p != you)
            && game.obj(d.source).def.supertypes.iter().any(|s| s == "LEGENDARY")
            && game.has_type(d.source, CardType::Creature)
    })
}
