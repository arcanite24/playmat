//! Skyserpent Seeker: "Flying, deathtouch. Exhaust — {4}: Reveal cards from the top of your library until you reveal
//! two land cards. Put those land cards onto the battlefield tapped and the rest on the bottom of your library in a
//! random order. Put a +1/+1 counter on this creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkyserpentSeeker.SkyserpentSeekerEffect", apply)];

/// the library from the top until two lands; Java reveals only the nonland cards, then the lands go onto the
/// battlefield tapped and the nonlands to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut revealed = Vec::new();
    let mut lands = Vec::new();
    for c in game.player(you).library.clone() {
        if game.obj(c).is(CardType::Land) {
            lands.push(c);
            if lands.len() == 2 {
                break;
            }
        } else {
            revealed.push(c);
        }
    }
    if !revealed.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = revealed.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    if !put_cards(game, d, you, &lands, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    if !put_cards(game, d, you, &revealed, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}
