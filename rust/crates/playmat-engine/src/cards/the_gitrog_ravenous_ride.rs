//! The Gitrog, Ravenous Ride: "Whenever The Gitrog deals combat damage to a player, you may sacrifice a creature that
//! saddled it this turn. If you do, draw X cards, then put up to X land cards from your hand onto the battlefield
//! tapped, where X is the sacrificed creature's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheGitrogRavenousRide.TheGitrogRavenousRideEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_LANDS"])];

/// `TargetSacrifice(0, 1, creatures that saddled it this turn)` (the saddle cost's marks, by The Gitrog as the
/// trigger was made: `SaddledSourceThisTurnPredicate`); none chosen, nothing. Its power now, the sacrifice, that many
/// cards drawn, then up to that many lands from hand (`TargetCardInHand`) put onto the battlefield tapped together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (mount, zcc) = cx.source;
    let mark = format!("saddled mount {} {} {}", mount.0, zcc, game.turn);
    let saddlers: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you
                && game.has_type(*o, CardType::Creature)
                && game.obj(*o).marks.contains(&mark)
        })
        .collect();
    let rule = item_rule(game, cx.item);
    let Some(victim) = choose_cards(game, d, you, &saddlers, &saddlers, 0, 1, &rule)?.first().copied() else {
        return Some(());
    };
    let power = game.power_toughness(victim).0;
    sacrifice(game, victim);
    if power > 0 {
        draw(game, you, power as usize);
    }
    let filter = cx.filters.first()?;
    let hand = game.player(you).hand.clone();
    let lands: Vec<ObjId> =
        hand.iter().copied().filter(|c| game.filter_matches(filter, you, Some(mount), *c)).collect();
    let max = power.max(0) as usize;
    let chosen = choose_cards(game, d, you, &hand, &lands, 0, max.min(lands.len()), &rule)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    Some(())
}
