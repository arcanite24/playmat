//! Krenko, Baron of Tin Street: "Whenever an artifact is put into a graveyard from the battlefield, you may pay {R}. If
//! you do, create a 1/1 red Goblin creature token. It gains haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KrenkoBaronOfTinStreet.KrenkoBaronOfTinStreetEffect", apply)
    .effects(&[
        "new CreateTokenEffect(new GoblinToken())",
        "new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.EndOfTurn)",
    ])];

/// the tokens made (doubled ones too) gain haste
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<Target> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(Target::Obj)
        .collect();
    if !made.is_empty() {
        cx.run_on(game, d, &cx.effects[1..2], made);
    }
    Some(())
}
