//! Ilysian Caryatid: "{T}: Add one mana of any color. If you control a creature with power 4 or greater, add two mana
//! of any one color instead." (`ConditionalManaEffect` of two `AddManaOfAnyColorEffect`s, `FerociousCondition`)

use super::*;
use crate::card::Mana;

pub const HANDLERS: &[Handler] = &[Handler::mana("IlysianCaryatid.ConditionalManaEffect", net, produce)];

/// `FerociousCondition`: two with a creature of yours with power 4 or greater, else one
fn amount(game: &Game, cx: &ManaCx) -> u32 {
    let ferocious = game.battlefield.iter().any(|o| {
        game.obj(*o).controller == cx.seat && game.has_type(*o, CardType::Creature) && game.power_toughness(*o).0 >= 4
    });
    if ferocious {
        2
    } else {
        1
    }
}

fn net(game: &Game, cx: &ManaCx) -> Mana {
    Mana { colored: [amount(game, cx); 5], ..Mana::default() }
}

/// `AddManaOfAnyColorEffect.produceMana`: "Select a color of mana to add N of it"
fn produce(game: &mut Game, d: &mut dyn Decider, cx: &ManaCx) -> Option<Mana> {
    let n = amount(game, cx);
    let c = any_color(game, d, cx.seat, cx.unpaid, n)?;
    Some(Mana::of(c, n))
}
