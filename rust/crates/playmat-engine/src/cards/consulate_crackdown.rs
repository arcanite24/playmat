//! Consulate Crackdown: "When Consulate Crackdown enters the battlefield, exile all artifacts your opponents control
//! until Consulate Crackdown leaves the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ConsulateCrackdown.ConsulateCracksownExileEffect", apply)];

/// With this still on the battlefield: every artifact an opponent controls, in battlefield order, exiled together
/// into this one's zone, back when it leaves (`OnLeaveReturnExiledAbility`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let you = cx.controller();
    let exiled: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller != you && game.has_type(*o, CardType::Artifact))
        .collect();
    let (src, zcc) = cx.source;
    for o in exiled {
        game.move_to(o, Zone::Exile);
        if game.obj(o).zone == Zone::Exile {
            game.exiled_until_leaves.push((src, zcc, o));
        }
    }
    Some(())
}
