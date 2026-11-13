//! Noosegraf Mob: "Whenever a player casts a spell, remove a +1/+1 counter from Noosegraf Mob. If you do, create a
//! 2/2 black Zombie creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NoosegrafMob.NoosegrafMobEffect", apply)
    .effects(&["new CreateTokenEffect(new ZombieToken())"])];

/// With the Mob on the battlefield (`getPermanent(sourceId)`) and a +1/+1 counter on it: one counter removed, then
/// a Zombie
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield || game.obj(src).counter("+1/+1") <= 0 {
        return Some(());
    }
    let o = game.obj_mut(src);
    o.add_counters("+1/+1", -1);
    o.counters.retain(|(_, k)| *k > 0);
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
