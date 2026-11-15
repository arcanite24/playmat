//! Rags // Riches: Riches's "Each opponent chooses a creature they control. You gain control of those creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RagsRiches.RichesEffect", riches)];

/// each opponent in turn order chooses one of their creatures (`opponent.choose`, not targeted); then you gain
/// control of each, for the rest of the game (`GainControlTargetEffect(EndOfGame)` with a fixed target)
fn riches(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut stolen = Vec::new();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let creatures = controlled_creatures(game, p);
        // (one with no creature isn't asked: `choose` finds no possible target)
        if creatures.is_empty() {
            continue;
        }
        let chosen = choose_cards(game, d, p, &creatures, &creatures, 1, 1, &rule)?;
        stolen.extend(chosen);
    }
    for o in stolen {
        let e = EffectDef::GainControlTargetEffect { duration: "EndOfGame".into() };
        cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    }
    Some(())
}
