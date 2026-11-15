//! Allure of the Unknown: "Reveal the top six cards of your library. An opponent exiles a nonland card from among
//! them, then you put the rest into your hand. That opponent may cast the exiled card without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AllureOfTheUnknown.AllureOfTheUnknownEffect", apply)];

/// The reveal; with no nonland card among them, all of them to your hand. Else an opponent chosen
/// (`TargetOpponent(true)`, asked even with one), who chooses a nonland card among them (`TargetCardInLibrary`, up
/// to one) to exile; the rest to your hand, then `castSpellWithAttributesForFree` for that opponent: "Cast spell
/// without paying its mana cost (…)?"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    if !cards.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: cards.iter().map(|c| (*c, Zone::Library)).collect(),
        });
    }
    let nonland: Vec<ObjId> = cards.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
    if nonland.is_empty() {
        if !put_cards(game, d, you, &cards, crate::card::PutCards::Hand) {
            return None;
        }
        return Some(());
    }
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    if opponents.is_empty() {
        return Some(());
    }
    // (`HumanPlayer.choose(Target)` asks even with one opponent)
    let opponent = loop {
        match ask(game, d, you, &Query::Target { players: opponents.clone(), objs: Vec::new(), required: true }) {
            Answer::Player(p) if opponents.contains(&p) => break p,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let rule = item_rule(game, cx.item);
    // (`TargetCardInLibrary` with a filter that has predicates isn't required: "done" with none chosen exiles none)
    let exiled = choose_cards_with(game, d, opponent, &cards, &nonland, 1, 1, &rule, true)?.first().copied();
    if let Some(card) = exiled {
        game.move_to(card, Zone::Exile);
    }
    let rest: Vec<ObjId> = cards
        .iter()
        .copied()
        .filter(|c| Some(*c) != exiled || game.obj(*c).zone != Zone::Exile)
        .filter(|c| game.obj(*c).zone == Zone::Library)
        .collect();
    if !put_cards(game, d, you, &rest, crate::card::PutCards::Hand) {
        return None;
    }
    let Some(card) = exiled.filter(|c| game.obj(*c).zone == Zone::Exile) else { return Some(()) };
    let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
    if choose_use(game, d, opponent, &message)? {
        cast_free(game, d, opponent, card);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
