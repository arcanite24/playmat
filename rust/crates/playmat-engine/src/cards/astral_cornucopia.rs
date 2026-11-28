//! Astral Cornucopia: "{T}: Choose a color. Add one mana of that color for each charge counter on Astral
//! Cornucopia." (`AstralCornucopiaManaEffect`)

use super::*;
use crate::card::Mana;

pub const HANDLERS: &[Handler] = &[Handler::mana("AstralCornucopia.AstralCornucopiaManaEffect", net, produce)];

fn charge(game: &Game, cx: &ManaCx) -> u32 {
    game.obj(cx.source).counter("charge").max(0) as u32
}

fn net(game: &Game, cx: &ManaCx) -> Mana {
    Mana { colored: [charge(game, cx); 5], ..Mana::default() }
}

fn produce(game: &mut Game, d: &mut dyn Decider, cx: &ManaCx) -> Option<Mana> {
    let n = charge(game, cx);
    if n == 0 {
        return Some(Mana::default());
    }
    let c = any_color_asking(game, d, cx.seat, cx.unpaid, "Choose a color to add mana of that color")?;
    Some(Mana::of(c, n))
}
