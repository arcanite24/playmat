//! Enter the God-Eternals: "Enter the God-Eternals deals 4 damage to target creature and you gain life equal to the
//! damage dealt this way. Target player mills four cards. Amass Zombies 4."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EnterTheGodEternals.EnterTheGodEternalsEffect", apply)
    .effects(&["new MillCardsTargetEffect(4)", "new AmassEffect(4, SubType.ZOMBIE)"])];

/// The targets in order: the damage and the life it gives, the mill; then the amass
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) {
        let dealt = deal_damage(game, cx.source_id(), Target::Obj(o), 4, false);
        gain_life(game, you, dealt);
    }
    if let Some(p) = cx.player(1) {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Player(p)]);
        if d.stopped() {
            return None;
        }
    }
    cx.run_on(game, d, &cx.effects[1..], Vec::new());
    (!d.stopped()).then_some(())
}
