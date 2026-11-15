//! Kairi, the Swirling Sky: "Flying. Ward {3}. When Kairi dies, choose one — • Return any number of target nonland
//! permanents with total mana value 6 or less to their owners' hands. • Mill six cards, then return up to two instant
//! and/or sorcery cards from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::target(
        "KairiTheSwirlingSky.KairiTheSwirlingSkyTarget",
        "new TargetPermanent(0, Integer.MAX_VALUE, new FilterNonlandPermanent(), false)",
        can_target,
    ),
    Handler::effect("KairiTheSwirlingSky.KairiTheSwirlingSkyEffect", apply)
        .effects(&["new MillCardsControllerEffect(6)"]),
];

/// `CardUtil.checkCanTargetTotalValueLimit(…, getManaValue, 6)`
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    super::joshua_phoenixs_dominant::within_total_mana_value(game, id, 6)
}

/// Mill six, then `choose(graveyard, TargetCardInGraveyard(0, 2, instant or sorcery))` (nothing asked with an empty
/// graveyard), then those cards into your hand together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let graveyard = game.player(you).graveyard.clone();
    if graveyard.is_empty() {
        return Some(());
    }
    let spells: Vec<ObjId> = graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Instant) || game.obj(*c).is(CardType::Sorcery))
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &graveyard, &spells, 0, 2, &rule)?;
    if chosen.is_empty() {
        return Some(());
    }
    if !put_cards(game, d, you, &chosen, crate::card::PutCards::Hand) {
        return None;
    }
    Some(())
}
