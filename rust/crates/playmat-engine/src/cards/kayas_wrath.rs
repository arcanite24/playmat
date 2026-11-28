//! Kaya's Wrath: "Destroy all creatures. You gain life equal to the number of creatures you controlled that were
//! destroyed this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KayasWrath.KayasWrathEffect", apply)
    .effects(&["new DestroyAllEffect(StaticFilters.FILTER_PERMANENT_CREATURE)"])];

/// The engine's destroy-all; the creatures you controlled that left the battlefield for it count (an
/// indestructible or regenerated one stays), then the life (`GainLifeEffect(counter)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<(ObjId, u32)> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let n = mine.iter().filter(|(o, z)| game.obj(*o).zcc != *z || game.obj(*o).zone != Zone::Battlefield).count();
    gain_life(game, you, n as i32);
    Some(())
}
