//! Storm Herald: "Haste. When Storm Herald enters, return any number of Aura cards from your graveyard to the
//! battlefield attached to creatures you control. Exile those Auras at the beginning of your next end step. If those
//! Auras would leave the battlefield, exile them instead of putting them anywhere else."

use super::*;
use std::sync::Arc;

pub const HANDLERS: &[Handler] = &[Handler::effect("StormHerald.StormHeraldEffect", apply)];

/// The Aura cards in your graveyard that could enchant a creature you control; you choose any number of them
/// (`chooseTarget`, `TargetCard(0, MAX)`), then for each in turn a creature it can enchant (`TargetPermanent`, not
/// targeted, `Player.choose`) and it enters attached to it. Those that entered are exiled if they'd leave (until
/// they do: the next end step's delayed trigger exiles them) and at the beginning of the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures = |game: &Game| -> Vec<ObjId> {
        game.battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
            .collect()
    };
    let mine = creatures(game);
    let auras: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).def.has_subtype("Aura") && mine.iter().any(|o| can_attach(game, you, *c, *o)))
        .collect();
    if auras.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &auras, &auras, 0, auras.len(), &rule)?;
    let mut to_exile: Vec<(ObjId, u32)> = Vec::new();
    for aura in chosen {
        if game.obj(aura).zone != Zone::Graveyard {
            continue;
        }
        let fits: Vec<ObjId> = creatures(game).into_iter().filter(|o| can_attach(game, you, aura, *o)).collect();
        if fits.is_empty() {
            continue;
        }
        let Some(&to) = choose_cards_rounds(game, d, you, &fits, 1, 1)?.first() else { continue };
        game.attach_entering = Some((aura, to, game.obj(to).zcc));
        put_onto_battlefield(game, d, aura, you);
        game.attach_entering = None;
        if d.stopped() {
            return None;
        }
        if game.obj(aura).zone == Zone::Battlefield {
            game.obj_mut(aura).unearthed = true;
            to_exile.push((aura, game.obj(aura).zcc));
        }
    }
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, exile those Auras.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed: to_exile,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// an Aura card whose spell's target filter matches the permanent
fn can_attach(game: &Game, you: Seat, card: ObjId, to: ObjId) -> bool {
    game.obj(card).def.spell.targets.iter().any(|t| match t {
        TargetDef::TargetPermanent { filter, .. } => game.filter_matches(filter, you, Some(card), to),
        t => possible_targets(game, you, t, Some(card)).1.contains(&to),
    })
}
