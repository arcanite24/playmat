//! Infinite Coursework: "Enchant creature. When this Aura enters, tap enchanted creature. It becomes unprepared.
//! Enchanted creature loses all abilities and doesn't untap during its controller's untap step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InfiniteCoursework.InfiniteCourseworkEffect", apply)];

/// `attachedTo.setPrepared(false)`: a prepared creature's copy of its prepare spell leaves exile
/// (`removePreparedCopy`: to no zone)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((creature, zcc)) = game.obj(cx.source_id()).attached_to else { return Some(()) };
    if game.obj(creature).zone != Zone::Battlefield || game.obj(creature).zcc != zcc {
        return Some(());
    }
    let Some(copy) = game.obj_mut(creature).prepared_copy.take() else { return Some(()) };
    if game.obj(copy).zone == Zone::Exile {
        game.remove_from_zone(copy);
        let c = game.obj_mut(copy);
        c.zone = Zone::Outside;
        c.may_play = None;
    }
    Some(())
}
