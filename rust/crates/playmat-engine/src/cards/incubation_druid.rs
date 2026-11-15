//! Incubation Druid: "{T}: Add one mana of any type that a land you control could produce. If Incubation Druid has a
//! +1/+1 counter on it, add three mana of that type instead." (its own `AnyColorLandsProduceManaEffect`)

use super::*;
use crate::card::Mana;

pub const HANDLERS: &[Handler] = &[Handler::mana("IncubationDruid.AnyColorLandsProduceManaEffect", net, produce)];

/// `getManaAmount`: three with a +1/+1 counter on it, else one
fn amount(game: &Game, cx: &ManaCx) -> u32 {
    let o = game.obj(cx.source);
    if o.zone == Zone::Battlefield && o.counter("+1/+1") > 0 {
        3
    } else {
        1
    }
}

fn net(game: &Game, cx: &ManaCx) -> Mana {
    let n = amount(game, cx);
    let (colors, colorless) = lands_could_produce(game, cx.seat, cx.source, false);
    let mut m = Mana::default();
    for c in colors {
        m.colored[c.index()] = n;
    }
    if colorless {
        m.colorless = n;
    }
    m
}

/// "Pick a mana color" among the types the lands could make (Colorless too), auto-chosen while paying as Exotic
/// Orchard's is
fn produce(game: &mut Game, d: &mut dyn Decider, cx: &ManaCx) -> Option<Mana> {
    let n = amount(game, cx);
    let (colors, colorless) = lands_could_produce(game, cx.seat, cx.source, false);
    if colors.is_empty() && !colorless {
        return Some(Mana::default());
    }
    let unpaid = cx.unpaid.clone();
    Some(match mana_type_among(game, d, cx.seat, &unpaid, &colors, colorless, false)? {
        Some(c) => Mana::of(c, n),
        None => Mana { colorless: n, ..Mana::default() },
    })
}
