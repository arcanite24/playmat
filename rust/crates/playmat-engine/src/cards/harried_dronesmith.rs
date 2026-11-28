//! Harried Dronesmith: "At the beginning of combat on your turn, create a 1/1 colorless Thopter artifact creature
//! token with flying. It gains haste until end of turn. Sacrifice it at the beginning of your next end step."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("HarriedDronesmith.HarriedDronesmithEffect", apply).tokens(&["ThopterColorlessToken"])];

/// The Thopter (`token.putOntoBattlefield`), haste for it until end of turn, then a delayed "sacrifice it" at your
/// next end step: the trigger is on your turn, so that's this turn's
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before: Vec<ObjId> = game.battlefield.iter().copied().collect();
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: Amount::Fixed(1),
        tapped_and_attacking: false,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: true,
        also: None,
        attach_source: None,
    };
    cx.run(game, d, &[e]);
    if d.stopped() {
        return None;
    }
    let made: Vec<(ObjId, u32)> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).token && !before.contains(o))
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    let delayed = Delayed {
        rule: "At the beginning of your next end step, sacrifice it.".into(),
        sacrifice: made,
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: 0,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
