//! Aligned Hedron Network: "When Aligned Hedron Network enters the battlefield, exile all creatures with power 5 or
//! greater until Aligned Hedron Network leaves the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AlignedHedronNetwork.AlignedHedronNetworkExileEffect", apply)];

/// With this still on the battlefield: every creature with power 5 or more, in battlefield order, exiled together
/// into this one's zone, back when it leaves (`OnLeaveReturnExiledAbility`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let exiled: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Creature) && game.power_toughness(*o).0 > 4)
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
