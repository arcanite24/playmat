//! Flood of Tears: "Return all nonland permanents to their owners' hands. If you return four or more nontoken
//! permanents you control this way, you may put a permanent card from your hand onto the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FloodOfTears.FloodOfTearsEffect", apply)
    .effects(&["new PutCardFromHandOntoBattlefieldEffect()"])];

/// Every nonland permanent to its owner's hand together, then the put when four or more were your nontoken ones
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let nonlands: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| !game.has_type(*o, CardType::Land)).collect();
    if nonlands.is_empty() {
        return None;
    }
    let put = nonlands.iter().filter(|o| game.obj(**o).controller == you && !game.obj(**o).token).count() > 3;
    for o in nonlands {
        game.move_to(o, Zone::Hand);
    }
    if put {
        cx.run(game, d, &cx.effects[..1]);
    }
    Some(())
}
