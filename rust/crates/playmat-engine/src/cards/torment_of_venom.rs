//! Torment of Venom: "Put three -1/-1 counters on target creature. Its controller loses 3 life unless they sacrifice
//! another nonland permanent or discard a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TormentOfVenom.TormentOfVenomEffect", apply)];

/// the counters; then its controller, with a nonland permanent: "Sacrifices a nonland permanent?" and one chosen
/// (another nonland permanent they control) sacrificed; else with cards in hand "Discard a card?"; else 3 life lost
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    add_counters_by(game, creature, "-1/-1", 3, Some(cx.controller()));
    let p = game.obj(creature).controller;
    let nonland = |game: &Game, o: ObjId| game.obj(o).controller == p && !game.has_type(o, CardType::Land);
    let permanents = game.battlefield.iter().filter(|o| nonland(game, **o)).count();
    if permanents > 0 && choose_use(game, d, p, "Sacrifices a nonland permanent?")? {
        let others: Vec<ObjId> =
            game.battlefield.iter().copied().filter(|o| *o != creature && nonland(game, *o)).collect();
        if !others.is_empty() {
            let rule = item_rule(game, cx.item);
            if let Some(&o) = choose_cards(game, d, p, &others, &others, 1, 1, &rule)?.first() {
                if game.obj(o).zone == Zone::Battlefield {
                    sacrifice(game, o);
                    return Some(());
                }
            }
        }
    }
    if !game.player(p).hand.is_empty() && choose_use(game, d, p, "Discard a card?")? {
        discard(game, d, p, 1);
        return (!d.stopped()).then_some(());
    }
    lose_life_by(game, p, 3, Some(cx.source_id()), false);
    Some(())
}
