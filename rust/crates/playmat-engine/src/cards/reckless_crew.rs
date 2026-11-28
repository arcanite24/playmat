//! Reckless Crew: "Create X 2/1 red Dwarf Berserker creature tokens, where X is the number of Vehicles you control
//! plus the number of Equipment you control. For each of those tokens, you may attach an Equipment you control to
//! it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("RecklessCrew.RecklessCrewEffect", apply).tokens(&["DwarfBerserkerToken"])];

/// X Dwarves (none: nothing); then, with any Equipment, for each token made (`getLastAddedTokenIds`) still on the
/// battlefield, you choose up to one Equipment you control (`TargetPermanent(0, 1, …, true)` with `choose`) and it's
/// attached to that token (`addAttachment`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let yours = |game: &Game, sub: &str| -> Vec<ObjId> {
        game.battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, sub))
            .collect()
    };
    let equipment = yours(game, "Equipment").len();
    let vehicles = yours(game, "Vehicle").len();
    if equipment + vehicles < 1 {
        return Some(());
    }
    let first = game.objs.len();
    cx.create_tokens(game, d, &cx.tokens[0], (equipment + vehicles) as i32);
    if d.stopped() {
        return None;
    }
    if equipment < 1 {
        return Some(());
    }
    let made: Vec<ObjId> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for t in made {
        if game.obj(t).zone != Zone::Battlefield {
            continue;
        }
        let possible = yours(game, "Equipment");
        let message = format!("Equipment you control (to attach to {})", game.obj(t).def.name);
        let Some(&e) = choose_cards(game, d, you, &possible, &possible, 0, 1, &message)?.first() else {
            continue;
        };
        if game.obj(e).zone == Zone::Battlefield
            && e != t
            && game.attached(e) != Some(t)
            && !game.has_type(e, CardType::Creature)
            && !game.protected_from(t, e)
        {
            let z = game.obj(t).zcc;
            game.attach_to(e, t, z);
        }
    }
    Some(())
}
