//! Twisted Sewer-Witch: "When this creature enters, create a 1/1 black Rat creature token with \"This token can't
//! block.\" Then for each Rat you control, create a Wicked Role token attached to that Rat."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TwistedSewerWitch.TwistedSewerWitchEffect", apply)
    .effects(&["new CreateRoleAttachedTargetEffect(RoleType.WICKED)"])];

/// a Wicked Role on each Rat you control, in battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rats: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) && game.has_subtype(*o, "Rat")
        })
        .collect();
    for rat in rats {
        if game.obj(rat).zone == Zone::Battlefield {
            cx.run_on(game, d, cx.effects, vec![Target::Obj(rat)]);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
