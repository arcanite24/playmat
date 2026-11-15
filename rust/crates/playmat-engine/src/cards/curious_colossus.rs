//! Curious Colossus: "When this creature enters, each creature target opponent controls loses all abilities, becomes
//! a Coward in addition to its other types, and has base power and toughness 1/1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CuriousColossus.CuriousColossusEffect", apply).effects(&[
    "new AddCardSubTypeTargetEffect(SubType.COWARD, Duration.Custom)",
    "new SetBasePowerToughnessTargetEffect(1, 1, Duration.EndOfGame)",
])];

/// the creatures the target opponent controls now (`FixedTargets`), each for as long as it stays: loses all abilities
/// (`LoseAllAbilitiesTargetEffect(Duration.Custom)`), becomes a Coward, has base power and toughness 1/1
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    if creatures.is_empty() {
        return Some(());
    }
    for c in &creatures {
        let zcc = game.obj(*c).zcc;
        lose_all_abilities(game, *c, *c, zcc);
    }
    let targets: Vec<Target> = creatures.iter().map(|c| Target::Obj(*c)).collect();
    cx.run_on(game, d, cx.effects, targets);
    Some(())
}
