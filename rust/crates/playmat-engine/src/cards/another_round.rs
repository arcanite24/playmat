//! Another Round: "Exile any number of creatures you control, then return them to the battlefield under their
//! owner's control. Then repeat this process X more times."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AnotherRound.AnotherRoundEffect", apply)];

/// X + 1 times: `chooseTarget` of a not-targeted `TargetControlledCreaturePermanent(0, MAX)`, then
/// `ExileThenReturnTargetEffect(false, true)` on those chosen
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    for _ in 0..=cx.item.x.max(0) {
        let mine: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
            .collect();
        let chosen = choose_cards(game, d, you, &mine, &mine, 0, mine.len(), &rule)?;
        let targets: Vec<Target> = chosen.into_iter().map(Target::Obj).collect();
        cx.run_on(game, d, &[EffectDef::ExileThenReturnTargetEffect], targets);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
