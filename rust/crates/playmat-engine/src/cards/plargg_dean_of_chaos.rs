//! Plargg, Dean of Chaos // Augusta, Dean of Order. Plargg: "{4}{R}, {T}: Reveal cards from the top of your library
//! until you reveal a nonlegendary, nonland card with mana value 3 or less. You may cast that card without paying its
//! mana cost. Put all revealed cards not cast this way on the bottom of your library in a random order." Augusta:
//! "Whenever you attack, untap each creature you control, then tap any number of creatures you control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("PlarggDeanOfChaos.PlarggDeanOfChaosEffect", plargg),
    Handler::effect("PlarggDeanOfChaos.AugustaDeanOfOrderEffect", augusta),
];

/// The library revealed down to the first nonland, nonlegendary card with mana value under 4; "Cast <card> without
/// paying its mana cost?" and the free cast; the rest (and the card when it wasn't cast) on the bottom
fn plargg(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let library = game.player(you).library.clone();
    if library.is_empty() {
        return Some(());
    }
    let mut revealed = Vec::new();
    let mut hit = None;
    for c in library {
        revealed.push(c);
        let o = game.obj(c);
        if !game.has_type(c, CardType::Land) && !o.def.supertypes.iter().any(|s| s == "LEGENDARY")
            && o.def.mana_value() < 4
        {
            hit = Some(c);
            break;
        }
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown = revealed.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    if let Some(card) = hit {
        let message = format!("Cast {} without paying its mana cost?", log::object(game, card));
        if choose_use(game, d, you, &message)? {
            cast_free_face(game, d, you, card);
            if d.stopped() {
                return None;
            }
        }
    }
    let rest: Vec<ObjId> = revealed.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    Some(())
}

/// `TargetPermanent(0, any, untapped creature you control, notTarget)` chosen, then each one tapped
fn augusta(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) && !game.obj(*o).tapped)
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, usize::MAX, &rule)?;
    for o in chosen {
        tap(game, o);
    }
    Some(())
}
