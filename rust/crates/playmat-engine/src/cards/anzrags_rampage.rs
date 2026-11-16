//! Anzrag's Rampage: "Destroy all artifacts you don't control, then exile the top X cards of your library, where X
//! is the number of artifacts that were put into graveyards from the battlefield this turn. You may put a creature
//! card exiled this way onto the battlefield. It gains haste. Return it to your hand at the beginning of the next end
//! step."

use super::*;
use crate::state::Delayed;

pub const HANDLERS: &[Handler] = &[
    Handler::value("AnzragsRampage.AnzragsRampageValue", artifacts_died),
    Handler::effect("AnzragsRampage.AnzragsRampageEffect", apply),
];

/// `AnzragsRampageWatcher`: the artifacts put into a graveyard from the battlefield this turn (each dies event)
fn died(game: &Game) -> usize {
    game.watch.dies.iter().filter(|(_, artifact, _, _)| *artifact).count()
}

fn artifacts_died(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    died(game) as i32
}

/// The top X cards exiled; up to one creature card among those still in exile chosen (`TargetCardInExile(0, 1)`,
/// not targeted) and put onto the battlefield; once there it gains haste for as long as it stays
/// (`Duration.Custom`), and a delayed trigger returns it to its owner's hand at the beginning of the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = died(game);
    let top: Vec<ObjId> = game.player(you).library.iter().take(n).copied().collect();
    for c in &top {
        game.move_to(*c, Zone::Exile);
        if game.obj(*c).zone == Zone::Exile {
            game.obj_mut(*c).exiled_with = Some(cx.source);
        }
    }
    let exiled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    let creatures: Vec<ObjId> = exiled.iter().copied().filter(|c| game.obj(*c).def.is(CardType::Creature)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &exiled, &creatures, 0, 1, &rule)?;
    let Some(&card) = chosen.first() else { return Some(()) };
    put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield);
    if d.stopped() {
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
        until_end_of_turn: false,
        until_turn_of: None,
    });
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return it to your hand.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ReturnToHandTargetEffect],
        source_zcc: cx.source.1,
        fixed: vec![(card, zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
