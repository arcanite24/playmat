//! War Effort: "Creatures you control get +1/+0. Whenever you attack, create a 1/1 red Warrior creature token that's
//! tapped and attacking. Sacrifice it at the beginning of the next end step."

use super::*;
use std::sync::Arc;

pub const HANDLERS: &[Handler] = &[Handler::effect("WarEffort.WarEffortEffect", apply).tokens(&["RedWarriorToken"])];

/// the token, tapped and attacking, then `SacrificeTargetEffect("sacrifice it")` at the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: Amount::Fixed(1),
        tapped_and_attacking: true,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    };
    let before = game.objs.len();
    cx.run_on(game, d, &[e], Vec::new());
    if d.stopped() {
        return None;
    }
    let created: Vec<(ObjId, u32)> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    let delayed = Delayed {
        rule: "At the beginning of the next end step, sacrifice it.".into(),
        sacrifice: created,
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: 0,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
