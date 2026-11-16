//! Faerie Slumber Party: "Return all creatures to their owners' hands. For each opponent who controlled a creature
//! returned this way, you create two 1/1 blue Faerie creature tokens with flying and "This creature can block only
//! creatures with flying.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FaerieSlumberParty.FaerieSlumberPartyEffect", apply)
    .effects(&["new ReturnToHandFromBattlefieldAllEffect(StaticFilters.FILTER_PERMANENT_CREATURE)"])
    .tokens(&["FaerieBlockFliersToken"])];

/// The opponents controlling a creature are counted first (before anything returns), then every creature returns,
/// then two Faeries for each of those opponents
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut controllers: Vec<Seat> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| !game.obj(*o).phased_out && game.has_type(*o, CardType::Creature))
        .map(|o| game.obj(o).controller)
        .filter(|c| *c != you)
        .collect();
    controllers.sort();
    controllers.dedup();
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    if !controllers.is_empty() {
        cx.create_tokens(game, d, &cx.tokens[0], 2 * controllers.len() as i32);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
