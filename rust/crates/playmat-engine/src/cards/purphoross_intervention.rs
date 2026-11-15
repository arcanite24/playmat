//! Purphoros's Intervention: "Choose one — • Create an X/1 red Elemental creature token with trample and haste.
//! Sacrifice it at the beginning of the next end step. • Purphoros's Intervention deals twice X damage to target
//! creature or planeswalker."

use super::*;
use crate::state::Delayed;
use std::sync::Arc;

pub const HANDLERS: &[Handler] = &[Handler::effect("PurphorossIntervention.PurphorossInterventionEffect", apply)
    .tokens(&["PurphorossInterventionToken"])];

/// an X/1 token, then for each token made a delayed "sacrifice this creature" at the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut token = cx.tokens[0].clone();
    token.power = cx.item.x;
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.create_tokens(game, d, &token, 1);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| !before.contains(o) && game.obj(*o).token).collect();
    for t in made {
        let delayed = Delayed {
            rule: "At the beginning of the next end step, sacrifice this creature.".into(),
            sacrifice: vec![(t, game.obj(t).zcc)],
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
    }
    Some(())
}
