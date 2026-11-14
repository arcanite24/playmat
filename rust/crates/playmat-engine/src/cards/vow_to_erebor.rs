//! Vow to Erebor: "Untap target creature you control. It gets +2/+2 until end of turn. If it's a Dwarf, you may attach
//! an Equipment you control to it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VowToErebor.VowToEreborEffect", apply)];

/// a Dwarf with an Equipment of yours to attach: asked, then up to one Equipment (not targeted)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(creature) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    if !game.has_subtype(creature, "Dwarf") {
        return Some(());
    }
    let equipment: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Equipment"))
        .collect();
    if equipment.is_empty() {
        return Some(());
    }
    let name = game.obj(creature).name().to_string();
    if !choose_use(game, d, you, &format!("Attach an Equipment you control to {name}?"))? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &equipment, &equipment, 0, 1, &rule)?;
    let z = game.obj(creature).zcc;
    for e in chosen {
        if game.obj(e).zone == Zone::Battlefield && e != creature && game.attached(e) != Some(creature) {
            game.attach_to(e, creature, z);
        }
    }
    Some(())
}
