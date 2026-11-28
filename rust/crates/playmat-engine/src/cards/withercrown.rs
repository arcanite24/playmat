//! Withercrown: "Enchant creature. Enchanted creature has base power 0 and has 'At the beginning of your upkeep, you
//! lose 1 life unless you sacrifice this creature.'"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Withercrown.DoUnlessControllerPaysEffect", apply)
    .args(&["effect", "one_of:new SacrificeSourceCost()", "one_of:rule"])];

/// `DoUnlessControllerPaysEffect`: the controller may sacrifice the creature (`chooseUse(Outcome.Detriment, rule)`,
/// asked only when it's there to sacrifice); not paid, they lose 1 life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let can_pay = cx.source_on_battlefield(game) && game.obj(src).controller == you;
    if can_pay
        && choose_use(game, d, you, "Do you want to sacrifice the enchanted creature?  If not, you lose 1 life.")?
    {
        sacrifice(game, src);
        return Some(());
    }
    cx.run(game, d, cx.effects);
    (!d.stopped()).then_some(())
}
