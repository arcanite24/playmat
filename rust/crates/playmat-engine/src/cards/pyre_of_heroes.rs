//! Pyre of Heroes: "{2}, {T}, Sacrifice a creature: Search your library for a creature card that shares a creature
//! type with the sacrificed creature and has mana value equal to 1 plus that creature's mana value. Put that card
//! onto the battlefield, then shuffle. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PyreOfHeroes.PyreOfHeroesEffect", apply)];

/// The sacrificed creature (`SacrificeTargetCost.getPermanents`): a creature card of 1 more mana value sharing one
/// of its creature types (`SharesCreatureTypePredicate`), searched for, put onto the battlefield; then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let sacrificed = *cx.item.sacrificed.first()?;
    let n = cx.item.sacrificed_stats[2] + 1;
    let def = game.obj(sacrificed).def.clone();
    // a changeling shares a type with any card that has one: the types of the library's cards (a changeling card
    // there has every one, "Shapeshifter" among them)
    let types: Vec<String> = if def.all_creature_types {
        let mut ts = vec!["Shapeshifter".to_string()];
        for c in &game.player(you).library {
            ts.extend(game.obj(*c).def.subtypes.iter().cloned());
        }
        ts
    } else {
        def.subtypes.clone()
    };
    let shares = Pred::Or {
        xs: types
            .into_iter()
            .filter(|s| !crate::card::NON_CREATURE_TYPES.contains(&s.as_str()))
            .map(|s| Pred::Subtype { s })
            .collect(),
    };
    let filter = Filter(vec![Pred::Mv { cmp: Cmp::EqualTo, n }, Pred::Type { t: CardType::Creature }, shares]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&c) = found.first() {
        if game.obj(c).zone == Zone::Library {
            enter_battlefield(game, d, c, you, false);
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
