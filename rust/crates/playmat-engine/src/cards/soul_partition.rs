//! Soul Partition: "Exile target nonland permanent. For as long as that card remains exiled, its owner may play
//! it. A spell cast by an opponent this way costs {2} more to cast."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SoulPartition.SoulPartitionEffect", apply)];

/// Its owner may play it from exile; {2} more when the owner is your opponent
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    game.move_to(o, Zone::Exile);
    if game.obj(o).zone != Zone::Exile {
        return Some(());
    }
    let owner = game.obj(o).owner;
    let tax = if owner != cx.controller() { 2 } else { 0 };
    let c = game.obj_mut(o);
    c.may_play = Some((owner, u32::MAX));
    c.play_tax = tax;
    Some(())
}
