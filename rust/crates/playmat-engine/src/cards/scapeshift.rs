//! Scapeshift: "Sacrifice any number of lands. Search your library for up to that many land cards, put them onto the
//! battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Scapeshift.ScapeshiftEffect", apply)];

/// `choose(TargetSacrifice(0, any, lands))` (asked even with none), each sacrificed in the order chosen; then
/// `TargetCardInLibrary(n, lands)` (none to find when nothing was sacrificed), those onto the battlefield tapped,
/// and the library is shuffled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let lands: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Land))
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &lands, &lands, 0, lands.len().max(1), &rule)?;
    let mut amount = 0;
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
            amount += 1;
        }
    }
    if amount > 0 {
        let filter = crate::card::Filter(vec![crate::card::Pred::Type { t: CardType::Land }]);
        let found = search_library(game, d, you, &filter, amount, amount, cx.source_id())?;
        if !put_cards(game, d, you, &found, crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
