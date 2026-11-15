//! Bilbo, Thief in the Night: "Spells you cast from anywhere other than your hand cost {1} less to cast. Whenever
//! Bilbo attacks, you may cast an artifact, instant, or sorcery spell from your graveyard. If an instant or sorcery
//! spell cast this way would be put into your graveyard, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("SpellCastFromAnywhereOtherThanHand", not_from_hand),
    Handler::effect("BilboThiefInTheNight.BilboThiefInTheNightEffect", apply),
];

/// `SpellCastFromAnywhereOtherThanHand`: a card someone else owns, or one cast (or to be cast) from a zone other
/// than its owner's hand
fn not_from_hand(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    let from = if o.zone == Zone::Stack { o.cast_from } else { Some(o.zone) };
    o.owner != you || from != Some(Zone::Hand)
}

fn castable(game: &Game, c: ObjId) -> bool {
    let o = game.obj(c);
    o.is(CardType::Artifact) || o.is(CardType::Instant) || o.is(CardType::Sorcery)
}

/// With such a card in your graveyard: asked first, then one chosen (not targeted) and cast now, its costs paid; an
/// instant or sorcery is exiled instead of going to your graveyard this turn (`ThatSpellGraveyardExileReplacementEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).graveyard.iter().copied().filter(|c| castable(game, *c)).collect();
    if cards.is_empty() {
        return Some(());
    }
    if !choose_use(game, d, you, "Cast an artifact, instant, or sorcery spell from your graveyard?")? {
        return Some(());
    }
    let Some(&card) =
        choose_cards(game, d, you, &cards, &cards, 1, 1, "an artifact, instant, or sorcery card")?.first()
    else {
        return Some(());
    };
    let instant_or_sorcery = game.obj(card).is(CardType::Instant) || game.obj(card).is(CardType::Sorcery);
    cast_allowed(game, d, you, card);
    if d.stopped() {
        return None;
    }
    if instant_or_sorcery {
        game.exile_if_graveyard.push(card);
    }
    Some(())
}
