//! Nissa of Shadowed Boughs: "Landfall — Whenever a land you control enters, put a loyalty counter on Nissa of
//! Shadowed Boughs. +1: Untap target land you control. You may have it become a 3/3 Elemental creature with haste
//! and menace until end of turn. It's still a land. −5: You may put a creature card with mana value less than or
//! equal to the number of lands you control onto the battlefield from your hand or graveyard with two +1/+1
//! counters on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("NissaOfShadowedBoughs.NissaOfShadowedBoughsPutCardEffect", apply)];

/// The creature cards in your hand, then your graveyard, with mana value at most the lands you control; up to one
/// of them (`TargetCard(0, 1)`, `Player.choose`) onto the battlefield with two +1/+1 counters (`setEnterWithCounters`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let lands = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land))
        .count() as i32;
    let fits = |game: &Game, c: ObjId| {
        let o = game.obj(c);
        o.def.is(CardType::Creature) && (o.def.mana_value() as i32) <= lands
    };
    let mut cards: Vec<ObjId> = game.player(you).hand.iter().copied().filter(|c| fits(game, *c)).collect();
    cards.extend(game.player(you).graveyard.iter().copied().filter(|c| fits(game, *c)));
    if cards.is_empty() {
        return Some(());
    }
    let Some(&card) = choose_cards_rounds(game, d, you, &cards, 0, 1)?.first() else { return Some(()) };
    let zcc = game.obj(card).zcc;
    game.enter_counters.push((card, zcc, 2));
    put_onto_battlefield(game, d, card, you);
    game.enter_counters.retain(|(o, z, _)| !(*o == card && *z == zcc));
    (!d.stopped()).then_some(())
}
