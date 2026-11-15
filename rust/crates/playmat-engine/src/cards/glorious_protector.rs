//! Glorious Protector: "Flash. Flying. When Glorious Protector enters, you may exile any number of non-Angel
//! creatures you control until Glorious Protector leaves the battlefield. Foretell {2}{W}."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("GloriousProtector.GloriousProtectorEffect", apply).filters(&["filter"])];

/// With this still on the battlefield: `TargetPermanent(0, MAX, non-Angel creatures you control, true)` chosen with
/// `Player.choose`, the chosen exiled into this one's zone, back when it leaves (`OnLeaveReturnExiledAbility`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let you = cx.controller();
    let possible: Vec<ObjId> = matching(game, &cx.filters[0], you, cx.source_id())
        .into_iter()
        .filter(|o| game.obj(*o).controller == you)
        .collect();
    let rule = item_rule(game, cx.item);
    // (`HumanPlayer.choose` skips an empty choice only when it's required: with nothing to exile it still asks)
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, usize::MAX, &rule)?;
    let (src, zcc) = cx.source;
    for o in chosen {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        game.move_to(o, Zone::Exile);
        if game.obj(o).zone == Zone::Exile {
            game.exiled_until_leaves.push((src, zcc, o));
        }
    }
    Some(())
}
