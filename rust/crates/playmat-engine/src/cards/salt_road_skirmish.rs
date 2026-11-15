//! Salt Road Skirmish: "Destroy target creature. Create two 1/1 red Warrior creature tokens. They gain haste until
//! end of turn. Sacrifice them at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SaltRoadSkirmish.SaltRoadSkirmishEffect", apply).tokens(&["RedWarriorToken"])];

/// two Warriors with haste until end of turn, sacrificed at the next end step (its own delayed trigger:
/// `SacrificeTargetEffect("sacrifice them")`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: Amount::Fixed(2),
        tapped_and_attacking: false,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: true,
        also: None,
        attach_source: None,
    };
    let before = game.objs.len();
    cx.run_on(game, d, &[e], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<(ObjId, u32)> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, sacrifice them.".into(),
        sacrifice: made,
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
