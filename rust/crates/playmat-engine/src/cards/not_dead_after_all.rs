//! Not Dead After All: "Until end of turn, target creature you control gains "When this creature dies, return it to
//! the battlefield tapped under its owner's control, then create a Wicked Role token attached to it.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NotDeadAfterAll.NotDeadAfterAllEffect", apply)
    .effects(&["new CreateRoleAttachedTargetEffect(RoleType.WICKED)"])];

/// The dead card (the trigger's `SetTargetPointer.CARD`), if it's still in the graveyard, onto the battlefield
/// tapped under its owner's control; then a Wicked Role attached to the permanent it became
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let cards: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    for c in cards {
        let owner = game.obj(c).owner;
        if !put_cards(game, d, owner, &[c], crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
        if game.obj(c).zone == Zone::Battlefield {
            cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(c)]);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
