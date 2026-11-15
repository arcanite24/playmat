//! Masterful Replication: "Choose target artifact you control. Each other artifact you control becomes a copy of that
//! artifact until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MasterfulReplication.MasterfulReplicationEffect", apply)];

/// The target still on the battlefield: each other artifact you control, in battlefield order, becomes a copy of it
/// until end of turn (`game.copyPermanent(Duration.EndOfTurn, …)`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(from) = cx.obj(0) else { return Some(()) };
    if game.obj(from).zone != Zone::Battlefield {
        return Some(());
    }
    let others: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != from && game.obj(*o).controller == you && game.has_type(*o, CardType::Artifact))
        .collect();
    for to in others {
        let copy = game.obj(from).copiable();
        become_copy_until_end_of_turn(game, to, copy);
    }
    Some(())
}
