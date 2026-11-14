//! Bishop of Binding: "When Bishop of Binding enters the battlefield, exile target creature an opponent controls
//! until Bishop of Binding leaves the battlefield. Whenever Bishop of Binding attacks, target Vampire gets +X/+X until
//! end of turn, where X is the power of the exiled card."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("BishopOfBinding.BishopOfBindingExileEffect", exile),
    Handler::value("BishopOfBinding.BishopOfBindingValue", exiled_power),
];

/// With this still on the battlefield, the target exiled into its zone, back when it leaves
/// (`OnLeaveReturnExiledAbility`)
fn exile(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let (src, zcc) = cx.source;
    for o in cx.objs(0) {
        game.move_to(o, Zone::Exile);
        if game.obj(o).zone == Zone::Exile {
            game.exiled_until_leaves.push((src, zcc, o));
        }
    }
    Some(())
}

/// the power of the card in this permanent's exile zone (its card's printed power), 0 with none
fn exiled_power(game: &Game, _you: Seat, src: ObjId, _x: i32) -> i32 {
    let zcc = game.obj(src).zcc;
    game.exiled_until_leaves
        .iter()
        .find(|(s, z, c)| *s == src && *z == zcc && game.obj(*c).zone == Zone::Exile)
        .map(|(_, _, c)| game.obj(*c).def.power)
        .unwrap_or(0)
}
