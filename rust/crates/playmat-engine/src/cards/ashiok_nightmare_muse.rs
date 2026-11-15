//! Ashiok, Nightmare Muse: "+1: Create a 2/3 blue and black Nightmare creature token with "Whenever this creature
//! attacks or blocks, each opponent exiles the top two cards of their library." −3: Return target nonland permanent
//! to its owner's hand, then that player exiles a card from their hand. −7: You may cast up to three spells from
//! among face-up cards your opponents own from exile without paying their mana costs."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AshiokNightmareMuseToken.AshiokNightmareMuseTokenEffect", token_exile),
    Handler::effect("AshiokNightmareMuse.AshiokNightmareMuseBounceEffect", bounce),
    Handler::effect("AshiokNightmareMuse.AshiokNightmareMuseCastEffect", cast),
];

/// the top two cards of each opponent's library (in play), exiled together by the token's controller. Java gathers
/// them in a `HashSet`: here in turn order, each library from the top
fn token_exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    let mut cards = Vec::new();
    for k in 1..n {
        let p = (you + k) % n;
        if game.player(p).lost {
            continue;
        }
        cards.extend(game.player(p).library.iter().take(2).copied());
    }
    if !cards.is_empty() && !put_cards(game, d, you, &cards, crate::card::PutCards::Exiled) {
        return None;
    }
    Some(())
}

/// the target to its owner's hand (moved by the owner); then, with cards in hand, the owner exiles one
/// (`player.choose` of a required `TargetCardInHand`)
fn bounce(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    if game.obj(target).zone != Zone::Battlefield {
        return Some(());
    }
    let owner = game.obj(target).owner;
    if !put_cards(game, d, owner, &[target], crate::card::PutCards::Hand) {
        return None;
    }
    let hand = game.player(owner).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let chosen = choose_cards_rounds(game, d, owner, &hand, 1, 1)?;
    if !chosen.is_empty() && !put_cards(game, d, owner, &chosen, crate::card::PutCards::Exiled) {
        return None;
    }
    Some(())
}

/// `castMultipleWithAttributeForFree(…, face-up cards your opponents own in exile, any card, 3)`
fn cast(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.owner != you && !o.face_down
        })
        .collect();
    let max = cards.len().min(3);
    let mut cast_count = 0;
    cards.retain(|c| game.obj(*c).zone != Zone::Stack);
    let castable = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        cards.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect()
    };
    if castable(game, &cards).is_empty() {
        return Some(());
    }
    loop {
        let options = castable(game, &cards);
        let card = match options.len() {
            0 => None,
            1 => Some(options[0]),
            _ => {
                let rule = item_rule(game, cx.item);
                choose_cards(game, d, you, &options, &options, 0, 1, &rule)?.first().copied()
            }
        };
        let mut cast = false;
        if let Some(card) = card {
            let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
            if choose_use(game, d, you, &message)? {
                cast = cast_free(game, d, you, card);
                if d.stopped() {
                    return None;
                }
            }
        }
        cards.retain(|c| game.obj(*c).zone != Zone::Stack);
        if cards.is_empty() || castable(game, &cards).is_empty() {
            break;
        }
        if cast {
            cast_count += 1;
            if cast_count >= max {
                break;
            }
        } else {
            let message = format!("You can cast {} more spells for free. Continue casting?", max - cast_count);
            if !choose_use(game, d, you, &message)? {
                break;
            }
        }
    }
    Some(())
}
