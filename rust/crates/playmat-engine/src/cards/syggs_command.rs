//! Sygg's Command, the mode "Creatures target player controls gain lifelink until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SyggsCommand.SyggsCommandGainLifelinkEffect", apply)];

/// The player's creatures now (the effect locks its objects)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(player) = cx.player(0) else { return Some(()) };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|&id| game.obj(id).controller == player && game.has_type(id, CardType::Creature))
        .collect();
    for o in creatures {
        grant(game, o, &AbilityDef::LifelinkAbility);
    }
    Some(())
}
