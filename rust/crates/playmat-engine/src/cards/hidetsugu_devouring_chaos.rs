//! Hidetsugu, Devouring Chaos: "{B}, Sacrifice a creature: Scry 2. {2}{R}, {T}: Exile the top card of your library.
//! You may play that card this turn. When you exile a nonland card this way, Hidetsugu, Devouring Chaos deals damage
//! equal to the exiled card's mana value to any target."

use super::*;
use crate::card::TargetDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("HidetsuguDevouringChaos.HidetsuguDevouringChaosEffect", apply)];

/// Your top card exiled (`exileAndPlayFromExile(…, Duration.EndOfTurn, …)`): you may play it this turn; a nonland
/// card fires a reflexive trigger dealing its mana value to any target, with Java's generated text
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let &card = game.player(you).library.first()?;
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        let turn = game.turn;
        let o = game.obj_mut(card);
        o.may_play = Some((you, turn));
        o.may_play_end_step = false;
        o.may_play_while = None;
    }
    if game.has_type(card, CardType::Land) {
        return Some(());
    }
    let mv = game.obj(card).def.mana_value() as i32;
    let delayed = Delayed {
        rule: format!("{{this}} deals {mv} damage to any target."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(mv) }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![TargetDef::TargetAnyTarget {
            min: 1,
            max: 1,
            another: false,
            not_first: false,
            distribute: None,
            x_count: false,
            count: None,
        }],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
