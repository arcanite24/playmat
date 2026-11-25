//! Sorin, Grim Nemesis: "+1: Reveal the top card of your library and put that card into your hand. Each opponent
//! loses life equal to its mana value." and "−9: Create a number of 1/1 black Vampire Knight creature tokens with
//! lifelink equal to the highest life total among all players."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SorinGrimNemesis.SorinGrimNemesisRevealEffect", reveal),
    Handler::effect("SorinGrimNemesis.SorinTokenEffect", tokens).tokens(&["VampireKnightToken"]),
];

/// the top card revealed and put into your hand; once it's there, each opponent loses its mana value (none for 0)
fn reveal(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = *game.player(you).library.first()?;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc: cx.source.1,
        paren: false,
        cards: vec![(card, game.obj(card).zone)],
    });
    game.move_to(card, Zone::Hand);
    if game.obj(card).zone != Zone::Hand {
        return Some(());
    }
    let mana_value = game.obj(card).def.mana_value() as i32;
    if mana_value > 0 {
        let n = game.seats();
        let opponents: Vec<Seat> = (0..n).map(|i| (you + i) % n).filter(|p| *p != you && !game.player(*p).lost).collect();
        for p in opponents {
            lose_life(game, p, mana_value);
        }
    }
    Some(())
}

/// the highest life total among the players in range, that many Vampire Knights
fn tokens(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let most = (0..game.seats()).filter(|p| !game.player(*p).lost).map(|p| game.player(p).life).max().unwrap_or(0);
    cx.create_tokens(game, d, cx.tokens.first()?, most.max(0));
    Some(())
}
