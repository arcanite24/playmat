//! U.S.Agent, John Walker: "When U.S.Agent enters, create a colorless Equipment artifact token named Sturdy Shield
//! with "Equipped creature gets +1/+2" and equip {2}. Attach it to U.S.Agent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("USAgentJohnWalker.USAgentJohnWalkerEffect", apply)
    .effects(&["new CreateTokenEffect(new SturdyShieldToken())"])];

/// `token.putOntoBattlefield(1)`, then, with U.S.Agent still the same permanent, `addAttachment` for each token made
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let (src, zcc) = cx.source;
    if game.obj(src).zone != Zone::Battlefield || game.obj(src).zcc != zcc {
        return Some(());
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for shield in made {
        game.attach_to(shield, src, zcc);
    }
    Some(())
}
