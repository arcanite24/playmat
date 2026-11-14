//! Banner of Kinship: "As this artifact enters, choose a creature type. This artifact enters with a fellowship
//! counter on it for each creature you control of the chosen type. Creatures you control of the chosen type get
//! +1/+1 for each fellowship counter on this artifact."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("BannerOfKinship.BannerOfKinshipEffect", enters_with),
    Handler::continuous("BannerOfKinship.BannerOfKinshipBoostEffect", yours_of_chosen_type, Layer::ModifyPt(fellowship)),
];

/// as it enters (after the type is chosen): a fellowship counter for each creature you control of that type
fn enters_with(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.source_id();
    let you = cx.controller();
    let Some(chosen) = game.obj(id).chosen_type.clone() else { return Some(()) };
    let n = game
        .battlefield
        .iter()
        .filter(|c| {
            let o = game.obj(**c);
            o.controller == you && game.has_type(**c, CardType::Creature) && game.has_subtype(**c, &chosen)
        })
        .count() as i32;
    if n > 0 {
        add_counters_by(game, id, "fellowship", n, Some(you));
    }
    Some(())
}

/// creatures you control of the chosen type
fn yours_of_chosen_type(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield
        && o.controller == cx.controller
        && game.has_type(id, CardType::Creature)
        && game.obj(cx.source).chosen_type.as_ref().is_some_and(|t| game.has_subtype(id, t))
}

/// +1/+1 for each fellowship counter on the Banner
fn fellowship(game: &Game, cx: &StaticCx, _: ObjId) -> (i32, i32) {
    let n = game.obj(cx.source).counter("fellowship");
    (n, n)
}
