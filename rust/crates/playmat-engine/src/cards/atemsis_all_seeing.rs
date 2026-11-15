//! Atemsis, All-Seeing: "Whenever Atemsis deals damage to an opponent, you may reveal your hand. If cards with at
//! least six different mana values are revealed this way, that player loses the game."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AtemsisAllSeeing.AtemsisAllSeeingEffect", apply)];

/// the hand revealed; six or more different mana values among it: the damaged opponent loses
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponent = cx.player(0)?;
    let hand = game.player(you).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: hand.iter().map(|c| (*c, Zone::Hand)).collect(),
        });
    }
    let mut values: Vec<u32> = hand.iter().map(|c| game.obj(*c).def.mana_value()).collect();
    values.sort_unstable();
    values.dedup();
    if values.len() > 5 {
        cx.run_as(game, d, &[EffectDef::LoseGameSourceControllerEffect], opponent);
    }
    (!d.stopped()).then_some(())
}
