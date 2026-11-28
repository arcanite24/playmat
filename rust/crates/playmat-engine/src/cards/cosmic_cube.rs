//! Cosmic Cube: "Whenever you attack, look at the top six cards of your library. You may cast a spell from among
//! them with mana value less than or equal to the greatest power among attacking creatures you control without paying
//! its mana cost. Put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CosmicCube.CosmicCubeEffect", apply)];

/// `CardUtil.castSpellWithAttributesForFree` over the top six with a mana value filter: the nonland cards that pass
/// it (one is taken as it is, more are chosen among, up to one), "Cast spell without paying its mana cost (…)?", then
/// the cards still in the library go to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let power = game
        .battlefield
        .iter()
        .filter(|o| {
            let ob = game.obj(**o);
            ob.controller == you && ob.attacking.is_some() && game.has_type(**o, CardType::Creature)
        })
        .map(|o| game.power_toughness(*o).0)
        .max()
        .unwrap_or(0);
    let top: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    let castable: Vec<ObjId> = top
        .iter()
        .copied()
        .filter(|c| !game.obj(*c).is(CardType::Land) && game.obj(*c).def.mana_value() as i32 <= power)
        .collect();
    let card = match castable.len() {
        0 => None,
        1 => Some(castable[0]),
        _ => {
            let rule = item_rule(game, cx.item);
            choose_cards(game, d, you, &castable, &castable, 0, 1, &rule)?.first().copied()
        }
    };
    if let Some(card) = card {
        let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
        if choose_use(game, d, you, &message)? {
            cast_free(game, d, you, card);
            if d.stopped() {
                return None;
            }
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}
