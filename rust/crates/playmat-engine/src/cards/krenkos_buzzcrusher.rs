//! Krenko's Buzzcrusher: "Flying, trample. When Krenko's Buzzcrusher enters, for each player, destroy up to one
//! nonbasic land that player controls. For each land destroyed this way, its controller may search their library
//! for a basic land card, put it onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KrenkosBuzzcrusher.KrenkosBuzzcrusherEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND_A"])];

/// For each player in range from you, you choose up to one nonbasic land they control (`chooseTarget`, not
/// targeted); then they're destroyed together; then, in that order, each destroyed land's controller may
/// ("Search your library for a basic land card?") search for one, put it onto the battlefield tapped and shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut chosen: Vec<ObjId> = Vec::new();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let lands: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| {
                game.obj(*o).controller == p
                    && game.has_type(*o, CardType::Land)
                    && !game.obj(*o).def.supertypes.iter().any(|s| s == "BASIC")
            })
            .collect();
        if let Some(&l) = choose_cards(game, d, you, &lands, &lands, 0, 1, &rule)?.first() {
            chosen.push(l);
        }
    }
    let mut destroyed: Vec<(ObjId, Seat)> = Vec::new();
    for l in chosen {
        if game.obj(l).zone != Zone::Battlefield {
            continue;
        }
        let (controller, zcc) = (game.obj(l).controller, game.obj(l).zcc);
        destroy(game, l);
        if game.obj(l).zcc != zcc || game.obj(l).zone != Zone::Battlefield {
            destroyed.push((l, controller));
        }
    }
    for (_, p) in destroyed {
        if game.player(p).lost {
            continue;
        }
        if !choose_use(game, d, p, "Search your library for a basic land card?")? {
            continue;
        }
        let found = search_library(game, d, p, &cx.filters[0], 1, 1, cx.source_id())?;
        if !found.is_empty() && !put_cards(game, d, p, &found, crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
        let mut library = game.player(p).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(p).library = library;
    }
    Some(())
}
