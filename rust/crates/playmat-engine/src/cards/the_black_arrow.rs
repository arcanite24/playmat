//! The Black Arrow: "Flash. When The Black Arrow enters, it deals 1 damage to any target. If a Dragon is dealt
//! damage this way, destroy it. Equipped creature gets +1/+1 and has reach. Equip {1}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheBlackArrow.TheBlackArrowEffect", apply)];

/// A permanent target: 1 damage (`permanent.damage`), then, damage dealt and it a Dragon, destroyed; a player
/// target: 1 damage
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    match cx.target(0)? {
        Target::Obj(o) => {
            if game.obj(o).zone != Zone::Battlefield {
                return Some(());
            }
            let dealt = deal_damage(game, src, Target::Obj(o), 1, false) > 0;
            if dealt && game.obj(o).zone == Zone::Battlefield && game.has_subtype(o, "Dragon") {
                destroy(game, o);
            }
        }
        Target::Player(p) => {
            deal_damage(game, src, Target::Player(p), 1, false);
        }
    }
    Some(())
}
