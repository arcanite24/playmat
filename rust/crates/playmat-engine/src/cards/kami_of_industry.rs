//! Kami of Industry: "When Kami of Industry enters the battlefield, return target artifact card with mana value 3 or
//! less from your graveyard to the battlefield. It gains haste. Sacrifice it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KamiOfIndustry.KamiOfIndustryEffect", apply)];

/// The card back, haste until end of turn (`GainAbilityTargetEffect`'s default), and the delayed "sacrifice it"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0)?;
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let you = cx.controller();
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(card).zcc;
    Arc::make_mut(&mut game.grants).push(crate::state::Grant {
        target: card,
        zcc,
        ability: AbilityDef::HasteAbility,
        until_end_of_turn: true,
        until_turn_of: None,
    });
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, sacrifice it.".into(),
        sacrifice: vec![(card, zcc)],
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
