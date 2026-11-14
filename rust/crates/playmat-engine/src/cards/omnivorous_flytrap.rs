//! Omnivorous Flytrap: "Delirium — Whenever Omnivorous Flytrap enters or attacks, if there are four or more card
//! types among cards in your graveyard, distribute two +1/+1 counters among one or two target creatures. Then if
//! there are six or more card types among cards in your graveyard, double the number of +1/+1 counters on those
//! creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("OmnivorousFlytrap.OmnivorousFlytrapEffect", apply),
    Handler::condition("OmnivorousFlytrap.OmnivorousFlytrapCondition", six_types),
];

/// `CardTypesInGraveyardCount.YOU >= 6`
fn six_types(game: &Game, you: Seat, source: ObjId) -> bool {
    game.value(&Amount::Of(crate::card::Value::CardTypesInYourGraveyard), you, source, 0) >= 6
}

/// Each target still there, in the order chosen, gets as many +1/+1 counters as it has
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let n = game.obj(o).counter("+1/+1");
        if n > 0 {
            add_counters_by(game, o, "+1/+1", n, Some(you));
        }
    }
    Some(())
}
