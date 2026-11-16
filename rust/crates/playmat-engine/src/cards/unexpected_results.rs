//! Unexpected Results: "Shuffle your library, then reveal the top card. If it's a nonland card, you may cast it
//! without paying its mana cost. If it's a land card, you may put it onto the battlefield and return Unexpected
//! Results to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnexpectedResults.UnexpectedResultEffect", apply)];

/// with cards in the library: shuffled, the top card revealed; a land: "Put <name> onto the battlefield?", then this
/// card to its owner's hand; else "Cast <name> without paying its mana cost?" (`chooseAbilityForCast(card, true)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    if game.player(you).library.is_empty() {
        return Some(());
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    let card = *game.player(you).library.first()?;
    let zcc = game.obj(me).zcc;
    game.revealed.push(crate::state::Revealed { source: me, zcc, paren: false, cards: vec![(card, Zone::Library)] });
    if game.obj(card).is(CardType::Land) {
        let message = format!("Put {} onto the battlefield?", game.obj(card).name());
        if choose_use(game, d, you, &message)? {
            if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
                return None;
            }
            if game.obj(me).zone == Zone::Stack && !game.obj(me).token {
                game.move_to(me, Zone::Hand);
            }
        }
    } else {
        let message = format!("Cast {} without paying its mana cost?", game.obj(card).name());
        if choose_use(game, d, you, &message)? {
            cast_free_face(game, d, you, card);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
