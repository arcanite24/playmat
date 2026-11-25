//! Master of Predicaments: "Whenever Master of Predicaments deals combat damage to a player, choose a card in your
//! hand. That player guesses whether the card's mana value is greater than 4. If the player guessed wrong, you may
//! cast the card without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MasterOfPredicaments.MasterOfPredicamentsEffect", apply)];

/// An empty hand, nothing; one card is taken as it is (`getRandom` of one), more are chosen among. The damaged
/// player asked "Is the chosen card's mana value greater than 4?"; a wrong guess on a nonland card offers "Cast {name}
/// without paying its mana cost?", and a yes casts it free (`chooseAbilityForCast(card, game, true)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    let card = match hand.len() {
        0 => return Some(()),
        1 => hand[0],
        _ => {
            let rule = item_rule(game, cx.item);
            choose_cards(game, d, you, &hand, &hand, 1, 1, &rule)?.first().copied()?
        }
    };
    let Some(them) = cx.player(0) else { return Some(()) };
    let mana_value = game.obj(card).def.mana_value();
    let greater = choose_use(game, d, them, "Is the chosen card's mana value greater than 4?")?;
    if log::kept(game) {
        let guess = if greater { "greater than 4" } else { "not greater than 4" };
        log::inform(game, format!("{} guessed that the chosen card's mana value is {guess}", log::player(them)));
    }
    let wrong = if greater { mana_value <= 4 } else { mana_value > 4 };
    if !wrong || game.obj(card).is(CardType::Land) {
        return Some(());
    }
    let message = format!("Cast {} without paying its mana cost?", game.obj(card).name());
    if choose_use(game, d, you, &message)? {
        cast_free_face(game, d, you, card);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
