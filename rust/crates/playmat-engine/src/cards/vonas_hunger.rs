//! Vona's Hunger: "Ascend. Each opponent sacrifices a creature. If you have the city's blessing, instead each
//! opponent sacrifices half the creatures they control, rounded up."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VonasHunger.VonasHungerEffect", apply)];

/// each opponent chooses half their creatures, rounded up (`TargetSacrifice`), then all are sacrificed together:
/// in a duel, the one opponent's `SacrificeOpponentsEffect` of that many
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let opponent = game.opponent(cx.controller());
    let creatures = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == opponent && game.has_type(**b, CardType::Creature))
        .count() as i32;
    let n = (creatures + 1) / 2;
    if n > 0 {
        let e = EffectDef::SacrificeOpponentsEffect {
            filter: Filter::of(vec![Pred::Type { t: CardType::Creature }]),
            amount: Amount::Fixed(n),
        };
        cx.run(game, d, &[e]);
    }
    Some(())
}
