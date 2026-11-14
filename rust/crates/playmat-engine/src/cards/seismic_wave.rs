//! Seismic Wave: "Seismic Wave deals 2 damage to any target and 1 damage to each nonartifact creature target
//! opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SeismicWave.SeismicWaveEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(to) = cx.target(0) {
        deal_damage(game, cx.source_id(), to, 2, false);
    }
    let Some(p) = cx.player(1) else { return Some(()) };
    let hit: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|c| {
            game.obj(*c).controller == p
                && game.has_type(*c, CardType::Creature)
                && !game.has_type(*c, CardType::Artifact)
        })
        .collect();
    for c in hit {
        deal_damage(game, cx.source_id(), Target::Obj(c), 1, false);
    }
    Some(())
}
