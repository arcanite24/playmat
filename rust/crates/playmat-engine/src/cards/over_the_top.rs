//! Over the Top: "Each player reveals a number of cards from the top of their library equal to the number of nonland
//! permanents they control, puts all permanent cards they revealed this way onto the battlefield, and puts the rest
//! into their graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OverTheTop.OverTheTopEffect", apply)];

/// The nonland permanents each player controls are counted first; then each player in turn order from you reveals,
/// puts the permanent cards onto the battlefield and the rest still in the library into the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let seats = game.seats();
    let counts: Vec<usize> = (0..seats)
        .map(|p| {
            game.battlefield
                .iter()
                .filter(|o| game.obj(**o).controller == p && !game.has_type(**o, CardType::Land))
                .count()
        })
        .collect();
    for i in 0..seats {
        let p = (you + i) % seats;
        if game.player(p).lost || counts[p] < 1 {
            continue;
        }
        let cards: Vec<ObjId> = game.player(p).library.iter().take(counts[p]).copied().collect();
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
        let permanents: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).def.is_permanent_card()).collect();
        if !permanents.is_empty() && !put_cards(game, d, p, &permanents, crate::card::PutCards::Battlefield) {
            return None;
        }
        let rest: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).zone == Zone::Library).collect();
        put_cards(game, d, p, &rest, crate::card::PutCards::Graveyard);
    }
    Some(())
}
