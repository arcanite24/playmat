//! Origin of the Avengers, chapter II: "You may put a Hero creature card with mana value 3 or less from your hand
//! onto the battlefield. If you don't, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OriginOfTheAvengers.OriginOfTheAvengersEffect", apply)];

/// `chooseUse`, then one Hero creature card with mana value 3 or less from your hand (`TargetCardInHand(filter)`)
/// onto the battlefield; with none put there, you draw a card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut put = false;
    let msg = "Put Hero creature card with mana value 3 or less from your hand onto the battlefield?";
    if choose_use(game, d, you, msg)? {
        let hand = game.player(you).hand.clone();
        let heroes: Vec<ObjId> = hand
            .iter()
            .copied()
            .filter(|c| {
                game.has_type(*c, CardType::Creature)
                    && game.has_subtype(*c, "Hero")
                    && game.obj(*c).def.mana_value() <= 3
            })
            .collect();
        if !heroes.is_empty() {
            let rule = item_rule(game, cx.item);
            let chosen = choose_cards(game, d, you, &hand, &heroes, 1, 1, &rule)?;
            if let Some(&card) = chosen.first() {
                put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield);
                if d.stopped() {
                    return None;
                }
                put = game.obj(card).zone == Zone::Battlefield;
            }
        }
    }
    if !put {
        draw(game, you, 1);
    }
    Some(())
}
