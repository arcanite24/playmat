//! Titan of Industry's fourth mode: "Put a shield counter on a creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TitanOfIndustry.TitanOfIndustryEffect", apply)];

/// with a creature you control, one chosen (`TargetControlledCreaturePermanent`, not targeted, `Player.choose`) gets
/// a shield counter
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let can: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| game.obj(*p).controller == you && game.has_type(*p, CardType::Creature))
        .collect();
    if can.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &can, &can, 1, 1, "")?;
    if let Some(c) = chosen.first() {
        add_counters_by(game, *c, "shield", 1, Some(you));
    }
    Some(())
}
