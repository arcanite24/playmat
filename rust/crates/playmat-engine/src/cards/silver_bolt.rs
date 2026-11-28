//! Silver Bolt: "{3}, {T}, Sacrifice Silver Bolt: It deals 3 damage to target creature. If a Werewolf is dealt damage
//! this way, destroy it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SilverBolt.SilverBoltEffect", apply)];

/// The damage, then the destroy of a Werewolf that was dealt some
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    if deal_damage(game, cx.source_id(), Target::Obj(o), 3, false) > 0
        && game.obj(o).zone == Zone::Battlefield
        && game.has_subtype(o, "Werewolf")
    {
        destroy(game, o);
    }
    Some(())
}
