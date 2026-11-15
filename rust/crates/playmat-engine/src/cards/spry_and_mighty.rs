//! Spry and Mighty: "Choose exactly two creatures you control. You draw X cards and the chosen creatures get +X/+X and
//! gain trample until end of turn, where X is the difference between the chosen creatures' powers."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpryAndMighty.SpryAndMightyEffect", apply)
    .effects(&["new GainAbilityTargetEffect(TrampleAbility.getInstance(), Duration.EndOfTurn)"])];

/// `choose(TargetControlledCreaturePermanent(2))` (not targeted), with two or more; the draw and boost only for a
/// difference, the trample always
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    if mine.len() < 2 {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &mine, &mine, 2, 2, &rule)?;
    let chosen: Vec<ObjId> = chosen.into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    if chosen.len() != 2 {
        return Some(());
    }
    let diff = (game.power_toughness(chosen[0]).0 - game.power_toughness(chosen[1]).0).abs();
    let targets: Vec<Target> = chosen.iter().map(|o| Target::Obj(*o)).collect();
    if diff > 0 {
        draw(game, you, diff as usize);
        let e = EffectDef::BoostTargetEffect {
            power: Amount::Fixed(diff),
            toughness: Amount::Fixed(diff),
            duration: crate::card::Duration::EndOfTurn,
        };
        cx.run_on(game, d, &[e], targets.clone());
        if d.stopped() {
            return None;
        }
    }
    cx.run_on(game, d, cx.effects, targets);
    (!d.stopped()).then_some(())
}
