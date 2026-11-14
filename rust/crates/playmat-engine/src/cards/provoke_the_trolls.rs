//! Provoke the Trolls: "Provoke the Trolls deals 3 damage to any target. If a creature is dealt damage this way, it
//! gets +5/+0 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ProvokeTheTrolls.ProvokeTheTrollsEffect", apply)];

/// The 3 damage; a creature dealt some gets +3/+0 until end of turn (Java's `BoostTargetEffect(3, 0)`, whatever the
/// text says)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    match cx.target(0)? {
        Target::Player(p) => {
            deal_damage(game, cx.source_id(), Target::Player(p), 3, false);
        }
        Target::Obj(o) => {
            if game.obj(o).zone != Zone::Battlefield {
                return None;
            }
            if deal_damage(game, cx.source_id(), Target::Obj(o), 3, false) < 1 {
                return None;
            }
            if game.obj(o).zone == Zone::Battlefield && game.has_type(o, CardType::Creature) {
                boost(game, o, 3, 0, crate::card::Duration::EndOfTurn);
            }
        }
    }
    Some(())
}
