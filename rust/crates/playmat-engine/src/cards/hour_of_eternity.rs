//! Hour of Eternity: "Exile X target creature cards from your graveyard. For each card exiled this way, create a
//! token that's a copy of that card, except it's a 4/4 black Zombie."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HourOfEternity.HourOfEternityEffect", apply)];

/// The targets still in your graveyard into exile together, then for each that's in exile a token copy of it with
/// its power/toughness set to 4/4 (its characteristic-defining P/T removed), black, and a Zombie as its only
/// creature type
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = cx
        .objs(0)
        .into_iter()
        .filter(|c| game.obj(*c).zone == Zone::Graveyard && game.obj(*c).owner == you)
        .collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let copy: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "CreateTokenCopyTargetEffect",
        "power": 4,
        "toughness": 4,
        "only_subtype": "Zombie",
        "only_color": ["B"],
    }))
    .ok()?;
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            cx.run_on(game, d, std::slice::from_ref(&copy), vec![Target::Obj(c)]);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
