//! The Long Reach of Night, chapters I and II: "Each opponent sacrifices a creature of their choice unless they
//! discard a card." (`SacrificeOpponentsUnlessPayEffect(new DiscardCardCost(), FILTER_PERMANENT_CREATURE)`)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("TheLongReachOfNight.SacrificeOpponentsUnlessPayEffect", apply).args(&["text", "filter"])];

/// Each opponent in turn is asked "discard a card?" (the cost's text, not a mana cost); a yes discards a card of
/// their choice (`DiscardTargetCost.pay`: paid with one discarded). One who doesn't pay chooses a creature to
/// sacrifice (`TargetSacrifice(1, filter)`); the chosen are sacrificed together after everyone has answered.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.first()?;
    let n = game.seats();
    let opponents: Vec<Seat> = (0..n).map(|i| (you + i) % n).filter(|p| *p != you && !game.player(*p).lost).collect();
    let cost = "discard a card";
    let mut to_sacrifice = Vec::new();
    for p in opponents {
        let paid = if choose_use(game, d, p, &format!("{cost}?"))? {
            let hand = game.player(p).hand.clone();
            if hand.is_empty() {
                false
            } else {
                let chosen = choose_cards(game, d, p, &hand, &hand, 1, 1, "discard")?;
                for c in &chosen {
                    discard_card(game, p, *c);
                }
                if !chosen.is_empty() {
                    game.discarded_cards();
                }
                !chosen.is_empty()
            }
        } else {
            false
        };
        if paid {
            if log::kept(game) {
                log::inform(game, format!("{} chooses to pay {cost} to prevent the sacrifice effect", log::player(p)));
            }
            continue;
        }
        if log::kept(game) {
            log::inform(game, format!("{} chooses not to pay {cost} to prevent the sacrifice effect", log::player(p)));
        }
        let possible: Vec<ObjId> =
            matching(game, filter, p, cx.source_id()).into_iter().filter(|o| game.obj(*o).controller == p).collect();
        if possible.is_empty() {
            continue;
        }
        to_sacrifice.extend(choose_cards(game, d, p, &possible, &possible, 1, 1, "sacrifice")?);
    }
    for o in to_sacrifice {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    Some(())
}
