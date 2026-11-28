//! Legion Warboss: "At the beginning of combat on your turn, create a 1/1 red Goblin creature token. That token
//! gains haste until end of turn and attacks this combat if able."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("LegionWarboss.LegionWarbossEffect", apply).effects(&["new CreateTokenEffect(new GoblinToken())"])];

/// The tokens made (`getLastAddedTokenIds`) gain haste, then `LegionWarbossAbility` ("attacks this combat if
/// able") until end of combat. The engine's grants last until end of turn: the requirement only matters when
/// attackers are declared, so the one combat it's made for is the same.
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
    for t in &made {
        grant(game, *t, &AbilityDef::HasteAbility);
    }
    let attacks = AbilityDef::SimpleStaticAbility {
        effects: vec![EffectDef::AttacksIfAbleSourceEffect { condition: None }],
        rule: "This creature attacks this combat if able.".into(),
    };
    for t in &made {
        grant(game, *t, &attacks);
    }
    Some(())
}
