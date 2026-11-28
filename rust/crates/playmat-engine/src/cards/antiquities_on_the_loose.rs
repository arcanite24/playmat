//! Antiquities on the Loose: "Create two 2/2 red and white Spirit creature tokens. Then if this spell was cast from
//! anywhere other than your hand, put a +1/+1 counter on each Spirit you control. Flashback {4}{W}{W}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AntiquitiesOnTheLoose.AntiquitiesOnTheLooseEffect", apply)];

/// The spell's `getFromZone` isn't the hand: `AddCountersAllEffect(+1/+1, Spirit you control)`
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let spell = cx.item.card;
    if game.obj(spell).zone != Zone::Stack || cx.item.ability.is_some() || game.obj(spell).cast_from == Some(Zone::Hand)
    {
        return Some(());
    }
    let spirits: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Spirit"))
        .collect();
    for o in spirits {
        put_counters(game, o, "+1/+1", 1, you);
    }
    Some(())
}
