//! Desculpting Blast: "Return target nonland permanent to its owner's hand. If it was attacking, create a 1/1
//! colorless Drone artifact creature token with flying and "This token can block only creatures with flying.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DesculptingBlast.DesculptingBlastEffect", apply)
    .effects(&["new CreateTokenEffect(new DroneFlyingToken())"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let attacking = game.obj(o).attacking.is_some();
    game.move_to(o, Zone::Hand);
    if attacking {
        cx.run_on(game, d, &cx.effects[..1], Vec::new());
    }
    Some(())
}
