//! Cultivator Colossus: "When Cultivator Colossus enters the battlefield, you may put a land card from your hand
//! onto the battlefield tapped. If you do, draw a card and repeat this process."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CultivatorColossus.CultivatorColossusEffect", apply)];

/// While you have a land card in hand: you choose up to one (`TargetCardInHand(0, 1, lands)`); none chosen ends it.
/// It's put onto the battlefield tapped, and if it got there you draw a card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    loop {
        let lands: Vec<ObjId> =
            game.player(you).hand.iter().copied().filter(|c| game.has_type(*c, CardType::Land)).collect();
        if lands.is_empty() || game.player(you).lost {
            return Some(());
        }
        let hand = game.player(you).hand.clone();
        let chosen = choose_cards(game, d, you, &hand, &lands, 0, 1, &rule)?;
        let Some(&card) = chosen.first() else { return Some(()) };
        put_cards(game, d, you, &[card], crate::card::PutCards::BattlefieldTapped);
        if d.stopped() {
            return None;
        }
        if game.obj(card).zone == Zone::Battlefield {
            draw(game, you, 1);
        }
    }
}
