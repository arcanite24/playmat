//! Demonlord Belzenlok: "Flying, trample. When Demonlord Belzenlok enters, exile cards from the top of your library
//! until you exile a nonland card, then put that card into your hand. If the card's mana value is 4 or greater,
//! repeat this process. Demonlord Belzenlok deals 1 damage to you for each card put into your hand this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DemonlordBelzenlok.DemonlordBelzenlokEffect", apply)];

/// One card at a time from the top into exile; each nonland one goes to your hand, and one with mana value less than
/// 4 ends it; then the damage to you
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut to_hand = 0;
    while let Some(&card) = game.player(you).library.first() {
        game.move_to(card, Zone::Exile);
        if game.obj(card).zone == Zone::Library {
            break;
        }
        if !game.obj(card).is(CardType::Land) {
            let last = game.obj(card).def.mana_value() < 4;
            game.move_to(card, Zone::Hand);
            to_hand += 1;
            if last {
                break;
            }
        }
    }
    deal_damage(game, cx.source_id(), Target::Player(you), to_hand, false);
    Some(())
}
