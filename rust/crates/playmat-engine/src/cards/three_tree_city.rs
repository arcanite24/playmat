//! Three Tree City: "{2}, {T}: Choose a color. Add an amount of mana of that color equal to the number of creatures
//! you control of the chosen type." (`ThreeTreeCityManaAbility`, `ThreeTreeCityManaEffect`)

use super::*;
use crate::card::Mana;

pub const HANDLERS: &[Handler] = &[Handler::mana("ThreeTreeCity.ThreeTreeCityManaEffect", net, produce)];

/// creatures you control of the source's chosen type
fn creatures(game: &Game, cx: &ManaCx) -> u32 {
    let Some(t) = game.obj(cx.source).chosen_type.clone() else { return 0 };
    game.battlefield
        .iter()
        .filter(|o| {
            game.obj(**o).controller == cx.seat && game.has_type(**o, CardType::Creature) && game.has_subtype(**o, &t)
        })
        .count() as u32
}

fn net(game: &Game, cx: &ManaCx) -> Mana {
    let n = creatures(game, cx);
    Mana { colored: [n; 5], ..Mana::default() }
}

fn produce(game: &mut Game, d: &mut dyn Decider, cx: &ManaCx) -> Option<Mana> {
    let n = creatures(game, cx);
    if n == 0 {
        return Some(Mana::default());
    }
    let c = any_color_asking(game, d, cx.seat, cx.unpaid, "Choose the color of mana to add")?;
    Some(Mana::of(c, n))
}
