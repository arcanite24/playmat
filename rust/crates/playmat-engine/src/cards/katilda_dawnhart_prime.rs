//! Katilda, Dawnhart Prime: "Protection from Werewolves. Human creatures you control have "{T}: Add one mana of any of
//! this creature's colors." {4}{G}{W}, {T}: Put a +1/+1 counter on each creature you control."

use super::*;
use crate::card::Mana;

pub const HANDLERS: &[Handler] =
    &[Handler::mana("KatildaDawnhartPrime.KatildaDawnhartPrimeManaEffect", net, produce)];

/// the creature's colors as it is now, white, blue, black, red, green (none if it left)
fn colors(game: &Game, cx: &ManaCx) -> Vec<Color> {
    if game.obj(cx.source).zone != Zone::Battlefield {
        return Vec::new();
    }
    let have = game.colors(cx.source);
    Color::ALL.into_iter().filter(|c| have.contains(c)).collect()
}

/// `getNetMana`: one of each of its colors
fn net(game: &Game, cx: &ManaCx) -> Mana {
    let mut m = Mana::default();
    for c in colors(game, cx) {
        m.colored[c.index()] = 1;
    }
    m
}

/// "Pick a mana color" among its colors (`setManaColorChoice`: the one owed picks itself while paying); one color,
/// no question; none, no mana
fn produce(game: &mut Game, d: &mut dyn Decider, cx: &ManaCx) -> Option<Mana> {
    let choices = colors(game, cx);
    if choices.is_empty() {
        return Some(Mana::default());
    }
    let unpaid = cx.unpaid.clone();
    let c = pick_color(game, d, cx.seat, &unpaid, &choices, "Pick a mana color")?;
    Some(Mana::of(c, 1))
}
