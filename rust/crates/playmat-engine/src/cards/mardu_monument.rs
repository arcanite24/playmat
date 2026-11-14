//! Mardu Monument: "When this artifact enters, search your library for a basic Mountain, Plains, or Swamp card,
//! reveal it, put it into your hand, then shuffle. {2}{R}{W}{B}, {T}, Sacrifice this artifact: Create three 1/1 red
//! Warrior creature tokens. They gain menace and haste until end of turn. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MarduMonument.MarduMonumentEffect", apply)
    .effects(&["new CreateTokenEffect(new RedWarriorToken(), 3)"])];

/// The tokens, then menace and then haste on them until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for a in [AbilityDef::MenaceAbility, AbilityDef::HasteAbility] {
        for o in &made {
            grant(game, *o, &a);
        }
    }
    Some(())
}
