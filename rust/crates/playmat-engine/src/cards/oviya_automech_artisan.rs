//! Oviya, Automech Artisan: "Each creature that's attacking one of your opponents has trample. {G}, {T}: You may put
//! a creature or Vehicle card from your hand onto the battlefield. If you put an artifact onto the battlefield this
//! way, put two +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("OviyaAutomechArtisan.OviyaAutomechArtisanPredicate", attacking_an_opponent),
    Handler::effect("OviyaAutomechArtisan.OviyaAutomechArtisanEffect", apply),
];

/// `getCombat().getDefenderId(creature)` is one of your opponents (a creature attacking a planeswalker or a battle
/// has that permanent as its defender)
fn attacking_an_opponent(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield && o.attacking_pw.is_none() && o.attacking.is_some_and(|p| p != you)
}

/// `TargetCardInHand(0, 1, creature or Vehicle)` chosen with `Player.choose`; the card onto the battlefield, and an
/// artifact that entered gets two +1/+1 counters
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    let fits: Vec<ObjId> = hand
        .iter()
        .copied()
        .filter(|c| game.has_type(*c, CardType::Creature) || game.has_subtype(*c, "Vehicle"))
        .collect();
    // (nothing asked of an empty hand)
    if hand.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &hand, &fits, 0, 1, &rule)?.first() else { return Some(()) };
    put_onto_battlefield(game, d, card, you);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield && game.has_type(card, CardType::Artifact) {
        add_counters_by(game, card, "+1/+1", 2, Some(you));
    }
    Some(())
}
