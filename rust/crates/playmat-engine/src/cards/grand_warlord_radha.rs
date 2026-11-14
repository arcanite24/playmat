//! Grand Warlord Radha: "Whenever one or more creatures you control attack, add that much mana in any combination of
//! {R} and/or {G}. Until end of turn, you don't lose this mana as steps and phases end."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::effect("GrandWarlordRadha.GrandWarlordRadhaEffect", apply)];

/// the creatures you attack with, divided between {R} and {G} (`getMultiAmount`, all on {R} by default)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game
        .battlefield
        .iter()
        .filter(|o| {
            let p = game.obj(**o);
            p.zone == Zone::Battlefield && p.controller == you && p.attacking.is_some()
        })
        .count() as i32;
    if n < 1 {
        return Some(());
    }
    let values = multi_amount(game, d, you, vec![n, 0], vec![(0, n); 2], (n, n), (None, Vec::new()))?;
    let mut colored = [0u32; 5];
    colored[3] = values[0].max(0) as u32;
    colored[4] = values.get(1).copied().unwrap_or(0).max(0) as u32;
    cx.run(game, d, &[EffectDef::BasicManaEffect { colored, colorless: 0, until_end_of_turn: true }]);
    Some(())
}
