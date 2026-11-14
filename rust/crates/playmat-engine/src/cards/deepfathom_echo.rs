//! Deepfathom Echo: "At the beginning of combat on your turn, Deepfathom Echo explores. Then you may have it become
//! a copy of another creature you control until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeepfathomEcho.DeepfathomEchoEffect", apply)
    .filters(&["StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL"])];

/// up to one other creature you control (`chooseTarget` of a 0-1 not-target `TargetPermanent`), then
/// `game.copyPermanent(Duration.EndOfTurn, chosen, deepfathom, new EmptyCopyApplier())`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let src = cx.source_id();
    let possible = matching(game, &cx.filters[0], you, src);
    if possible.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, 1, "become a copy")?;
    let Some(of) = chosen.first().copied() else { return Some(()) };
    let copy = game.obj(of).copiable();
    become_copy_until_end_of_turn(game, src, copy);
    Some(())
}
