//! Shed Weakness: "Target creature gets +2/+2 until end of turn. You may remove a -1/-1 counter from it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShedWeakness.MayRemoveM1M1CouterTargetEffect", apply)];

/// the target still on the battlefield with a -1/-1 counter: "Remove a -1/-1 counter from <it [id]>?", and on a
/// yes one is removed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    if game.obj(o).counter("-1/-1") <= 0 {
        return Some(());
    }
    let r = crate::replay::reference(game, o);
    if choose_use(game, d, cx.controller(), &format!("Remove a -1/-1 counter from {} [{r}]?", game.obj(o).name()))? {
        let e = EffectDef::RemoveCounterTargetEffect { counter: "-1/-1".into() };
        cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    }
    Some(())
}
