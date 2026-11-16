//! Chandra, Acolyte of Flame: "0: Put a loyalty counter on each red planeswalker you control. 0: Create two 1/1 red
//! Elemental creature tokens. They gain haste. Sacrifice them at the beginning of the next end step. −2: You may cast
//! target instant or sorcery card with mana value 3 or less from your graveyard. If that card would be put into your
//! graveyard this turn, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ChandraAcolyteOfFlame.ChandraAcolyteOfFlameEffect", apply).tokens(&["RedElementalToken"])];

/// Both Elementals enter, then each in turn gains haste for as long as it stays (`Duration.Custom`) and gets its own
/// delayed "sacrifice it" (`SacrificeTargetEffect` at a fixed target)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.create_tokens(game, d, &cx.tokens[0], 2);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for id in made {
        let zcc = game.obj(id).zcc;
        Arc::make_mut(&mut game.grants).push(crate::state::Grant {
            target: id,
            zcc,
            ability: AbilityDef::HasteAbility,
            until_end_of_turn: false,
            until_turn_of: None,
        });
        let delayed = crate::state::Delayed {
            rule: "At the beginning of the next end step, that permanent's controller sacrifices it.".into(),
            sacrifice: vec![(id, zcc)],
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
