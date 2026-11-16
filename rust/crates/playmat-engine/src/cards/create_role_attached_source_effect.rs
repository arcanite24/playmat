//! `CreateRoleAttachedSourceEffect(RoleType.X)`: "create a Cursed Role token attached to it" (Cursed Courtier,
//! Faunsbane Troll, Unassuming Sage): the Role attached to the source, if it's still on the battlefield. The
//! transpiler hands the Role as a `CreateRoleAttachedTargetEffect`.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CreateRoleAttachedSourceEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    cx.run_on(game, d, cx.effects, vec![Target::Obj(cx.source_id())]);
    Some(())
}
