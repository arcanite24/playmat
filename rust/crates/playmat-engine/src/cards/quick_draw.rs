//! Quick Draw: "Target creature you control gets +1/+1 and gains first strike until end of turn. Creatures target
//! opponent controls lose first strike and double strike until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("QuickDraw.QuickDrawEffect", apply).effects(&[
    "new LoseAbilityTargetEffect(FirstStrikeAbility.getInstance(), Duration.EndOfTurn)",
    "new LoseAbilityTargetEffect(DoubleStrikeAbility.getInstance(), Duration.EndOfTurn)",
])];

/// The two `LoseAbilityAllEffect`s: their creatures are the opponent's as this resolves (a locked-in set)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(1)?;
    let theirs: Vec<Target> = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == p && game.has_type(**b, CardType::Creature))
        .map(|b| Target::Obj(*b))
        .collect();
    if !theirs.is_empty() {
        cx.run_on(game, d, cx.effects, theirs);
    }
    (!d.stopped()).then_some(())
}
