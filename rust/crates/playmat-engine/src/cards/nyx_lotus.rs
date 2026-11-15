//! Nyx Lotus: "Nyx Lotus enters the battlefield tapped. {T}: Choose a color. Add an amount of mana of that color
//! equal to your devotion to that color." (`NyxLotusManaAbility`, `NyxLotusDynamicManaEffect`)

use super::*;
use crate::card::Mana;

pub const HANDLERS: &[Handler] = &[Handler::mana("NyxLotus.NyxLotusDynamicManaEffect", net, produce)];

/// `getNetMana`: each color with devotion above 0, that much of it
fn net(game: &Game, cx: &ManaCx) -> Mana {
    let mut m = Mana::default();
    for c in Color::ALL {
        m.colored[c.index()] = devotion(game, cx.seat, c);
    }
    m
}

/// `produceMana`: "Choose a color for devotion of Nyx Lotus" (picked for you while paying when the unpaid cost
/// shows one), then the devotion to it
fn produce(game: &mut Game, d: &mut dyn Decider, cx: &ManaCx) -> Option<Mana> {
    let c = any_color_asking(game, d, cx.seat, cx.unpaid, "Choose a color for devotion of Nyx Lotus")?;
    Some(Mana::of(c, devotion(game, cx.seat, c)))
}
