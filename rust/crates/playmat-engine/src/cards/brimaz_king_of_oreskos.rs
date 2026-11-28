//! Brimaz, King of Oreskos: "Whenever Brimaz blocks a creature, create a 1/1 white Cat Soldier creature token with
//! vigilance that's blocking that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BrimazKingOfOreskos.BrimazKingOfOreskosEffect", apply)
    .effects(&["new CreateTokenEffect(new CatSoldierCreatureToken())"])];

/// The token is made first (outside combat or with the attacker gone, it just stays); then each token made blocks
/// the creature Brimaz blocked (`CombatGroup.addBlocker`, `Combat.addBlockingGroup`: no blocker-declared event)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let Some(attacker) = cx.obj(0) else { return Some(()) };
    if game.obj(attacker).zone != Zone::Battlefield || game.obj(attacker).attacking.is_none() {
        return Some(());
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for t in made {
        game.obj_mut(t).blocking.push(attacker);
        game.obj_mut(t).blocks += 1;
        let by = &mut game.obj_mut(attacker).blocked_by;
        by.retain(|x| *x != t);
        by.push(t);
        game.obj_mut(attacker).blocked = true;
    }
    Some(())
}
