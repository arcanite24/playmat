//! `ReturnToBattlefieldUnderOwnerControlSourceEffect([tapped[, -1]])`: "return it to the battlefield [tapped] under
//! its owner's control" (Tenacious Dead from its graveyard, Lamplight Phoenix from exile after its cost)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReturnToBattlefieldUnderOwnerControlSourceEffect", apply)
    .args(&["one_of:true|false", "one_of:-1"])];

/// The source card from a graveyard or exile (any zone change count: the constructors written here take -1), not
/// attacking
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.source_id();
    if game.obj(o).token || !matches!(game.obj(o).zone, Zone::Graveyard | Zone::Exile) {
        return None;
    }
    let owner = game.obj(o).owner;
    enter_battlefield(game, d, o, owner, false);
    if cx.args.first().is_some_and(|a| a == "true") && game.obj(o).zone == Zone::Battlefield {
        game.obj_mut(o).tapped = true;
    }
    Some(())
}
