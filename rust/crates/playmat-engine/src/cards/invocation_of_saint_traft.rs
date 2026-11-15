//! Invocation of Saint Traft: "Enchant creature. Enchanted creature has "Whenever this creature attacks, create a
//! 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InvocationOfSaintTraft.InvocationOfSaintTraftEffect", apply)
    .effects(&["new CreateTokenEffect(new AngelToken(), 1, true, true)"])];

/// The token, then `removeTokensCreatedAt(…, exile, END_COMBAT)`: an "at end of combat" delayed trigger exiling the
/// tokens made now that are on the battlefield (`ExileTargetEffect` with fixed targets)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let first = game.objs.len();
    cx.run(game, d, cx.effects);
    let made: Vec<(ObjId, u32)> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    let delayed = Delayed {
        rule: "At end of combat, exile those tokens.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed: made,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_end_of_combat
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
