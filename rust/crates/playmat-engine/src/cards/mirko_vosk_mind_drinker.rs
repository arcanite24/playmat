//! Mirko Vosk, Mind Drinker: "Flying. Whenever Mirko Vosk deals combat damage to a player, that player reveals cards
//! from the top of their library until they reveal four land cards, then puts those cards into their graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MirkoVoskMindDrinker.MirkoVoskMindDrinkerEffect", apply)];

/// The damaged player reveals down to the fourth land (or the whole library), and all of them go to the graveyard
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let mut lands = 4;
    let mut cards = Vec::new();
    for c in game.player(p).library.clone() {
        cards.push(c);
        if game.obj(c).is(CardType::Land) {
            lands -= 1;
            if lands < 1 {
                break;
            }
        }
    }
    if !cards.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    for c in cards {
        game.move_to(c, Zone::Graveyard);
    }
    Some(())
}
