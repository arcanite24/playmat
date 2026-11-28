//! Perforating Artist: "Raid — At the beginning of your end step, if you attacked this turn, each opponent loses 3
//! life unless that player sacrifices a nonland permanent of their choice or discards a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PerforatingArtist.PerforatingArtistEffect", apply)
    .filters(&["StaticFilters.FILTER_PERMANENT_NON_LAND"])];

/// Each opponent: with a nonland permanent, asked whether to sacrifice one (then chooses it); else, with a card in
/// hand, asked whether to discard; else loses 3 life. As Java's `apply`, a sacrifice or a discard ends the whole
/// effect (`return true`), so later opponents aren't asked.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let nonland: Vec<ObjId> = matching(game, &cx.filters[0], p, cx.source_id())
            .into_iter()
            .filter(|o| game.obj(*o).controller == p)
            .collect();
        if !nonland.is_empty() && choose_use(game, d, p, "Sacrifices a nonland permanent?")? {
            let rule = item_rule(game, cx.item);
            let chosen = choose_cards(game, d, p, &nonland, &nonland, 1, 1, &rule)?;
            if let Some(&o) = chosen.first() {
                if game.obj(o).zone == Zone::Battlefield {
                    sacrifice(game, o);
                    return Some(());
                }
            }
        }
        if !game.player(p).hand.is_empty() && choose_use(game, d, p, "Discard a card?")? {
            discard(game, d, p, 1);
            return if d.stopped() { None } else { Some(()) };
        }
        lose_life(game, p, 3);
    }
    Some(())
}
