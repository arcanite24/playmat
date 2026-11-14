//! Light of Judgment: "Light of Judgment deals 6 damage to target creature. Destroy up to one Equipment attached to
//! that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LightOfJudgment.LightOfJudgmentEffect", apply)];

/// The creature still on the battlefield with something attached: `choose` up to one Equipment attached to it (not
/// targeted), destroyed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield || game.attachments(t).next().is_none() {
        return Some(());
    }
    let equipment: Vec<ObjId> = game.attachments(t).filter(|a| game.has_subtype(*a, "Equipment")).collect();
    if equipment.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, cx.controller(), &equipment, &equipment, 0, 1, &rule)?;
    if let Some(&e) = chosen.first() {
        destroy(game, e);
    }
    Some(())
}
