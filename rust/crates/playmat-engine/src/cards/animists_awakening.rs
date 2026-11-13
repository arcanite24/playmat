//! Animist's Awakening: "Reveal the top X cards of your library. Put all land cards from among them onto the
//! battlefield tapped and the rest on the bottom of your library in a random order. Spell mastery — If there are two
//! or more instant and/or sorcery cards in your graveyard, untap those lands."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AnimistsAwakening.AnimistsAwakeningEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(cx.item.x.max(0) as usize).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let (src, zcc) = (cx.source_id(), game.obj(cx.source_id()).zcc);
    let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: shown });
    let (lands, rest): (Vec<ObjId>, Vec<ObjId>) = cards.into_iter().partition(|c| game.obj(*c).is(CardType::Land));
    if !put_cards(game, d, you, &lands, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    let mastery = game
        .player(you)
        .graveyard
        .iter()
        .filter(|c| game.obj(**c).is(CardType::Instant) || game.obj(**c).is(CardType::Sorcery))
        .count()
        >= 2;
    if mastery {
        for l in lands {
            if game.obj(l).zone == Zone::Battlefield {
                untap_permanent(game, l);
            }
        }
    }
    Some(())
}
