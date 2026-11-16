//! Lumbering Battlement: "Vigilance. When Lumbering Battlement enters, exile any number of other nontoken creatures
//! you control until it leaves the battlefield. Lumbering Battlement gets +2/+2 for each card exiled with it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LumberingBattlement.LumberingBattlementEffect", apply),
    Handler::value("LumberingBattlement.LumberingBattlementValue", twice_exiled),
];

/// With Lumbering Battlement still on the battlefield, you choose any number of your other nontoken creatures
/// (`TargetPermanent(0, MAX, filter, true)`, one `HumanPlayer.choose`, asked even with none); they're exiled
/// together into its zone, back when it leaves (`OnLeaveReturnExiledAbility`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let you = cx.controller();
    let (src, zcc) = cx.source;
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            *o != src && game.obj(*o).controller == you && !game.obj(*o).token && game.has_type(*o, CardType::Creature)
        })
        .collect();
    let rule = "exile any number of other nontoken creatures you control until it leaves the battlefield";
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, usize::MAX, rule)?;
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

/// twice the cards still in exile that this permanent exiled
fn twice_exiled(game: &Game, _you: Seat, source: ObjId, _x: i32) -> i32 {
    let zcc = game.obj(source).zcc;
    let n = game
        .exiled_until_leaves
        .iter()
        .filter(|(s, z, c)| *s == source && *z == zcc && game.obj(*c).zone == Zone::Exile)
        .count() as i32;
    2 * n
}
