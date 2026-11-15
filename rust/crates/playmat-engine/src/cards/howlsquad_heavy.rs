//! Howlsquad Heavy: "Start your engines! Other Goblins you control have haste. At the beginning of combat on your
//! turn, create a 1/1 red Goblin creature token. That token attacks this combat if able. Max speed — {T}: Add {R}
//! for each Goblin you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HowlsquadHeavy.HowlsquadHeavyEffect", apply)
    .effects(&["new CreateTokenEffect(new GoblinToken())"])];

/// `token.putOntoBattlefield`, then `AttacksIfAbleTargetEffect(Duration.EndOfCombat)` on the tokens made
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<Target> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(Target::Obj)
        .collect();
    if !made.is_empty() {
        cx.run_on(game, d, &[EffectDef::AttacksIfAbleTargetEffect { end_of_combat: true }], made);
    }
    Some(())
}
