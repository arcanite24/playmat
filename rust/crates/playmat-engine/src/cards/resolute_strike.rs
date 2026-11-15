//! Resolute Strike: "Target creature gets +2/+2 until end of turn. If it's a Warrior, you may attach an Equipment
//! you control to it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ResoluteStrike.ResoluteStrikeEffect", apply)];

/// A Warrior target and an Equipment you control: "Attach an equipment you control?", then up to one Equipment
/// (`TargetPermanent(0, 1, filter, true)` with `Player.choose`) attached to it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(creature) = cx.obj(0) else { return Some(()) };
    if game.obj(creature).zone != Zone::Battlefield || !game.has_subtype(creature, "Warrior") {
        return Some(());
    }
    let equipment: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Equipment"))
        .collect();
    if equipment.is_empty() || !choose_use(game, d, you, "Attach an equipment you control?")? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &equipment, &equipment, 0, 1, &rule)?;
    if let Some(&e) = chosen.first() {
        if e != creature && game.attached(e) != Some(creature) {
            let z = game.obj(creature).zcc;
            game.attach_to(e, creature, z);
        }
    }
    Some(())
}
