//! Living Lore: "As Living Lore enters, exile an instant or sorcery card from your graveyard. Living Lore's power and
//! toughness are each equal to the exiled card's mana value. Whenever Living Lore deals combat damage, you may
//! sacrifice it. If you do, you may cast the exiled card without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LivingLore.LivingLoreExileEffect", exile),
    Handler::value("LivingLore.LivingLoreValue", exiled_mana_value),
    Handler::effect("LivingLore.LivingLoreCastEffect", cast_exiled),
];

/// the cards in the exile zone of `source` as the permanent it is (or was: `zcc`)
fn exiled_with(game: &Game, source: ObjId, zcc: u32) -> Vec<ObjId> {
    game.exile.iter().copied().filter(|c| game.obj(*c).exiled_with == Some((source, zcc))).collect()
}

/// `TargetCardInYourGraveyard(instant or sorcery)`, not targeted, chosen with `chooseTarget`; exiled into the zone of
/// the permanent entering
fn exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let can: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Instant) || game.obj(*c).is(CardType::Sorcery))
        .collect();
    if can.is_empty() {
        return Some(());
    }
    let Some(card) = choose_cards(game, d, you, &can, &can, 1, 1, "exile")?.first().copied() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        game.obj_mut(card).exiled_with = Some(cx.source);
    }
    Some(())
}

/// the total mana value of the cards exiled with the permanent (0 off the battlefield)
fn exiled_mana_value(game: &Game, _: Seat, source: ObjId, _: i32) -> i32 {
    let o = game.obj(source);
    if o.zone != Zone::Battlefield {
        return 0;
    }
    exiled_with(game, source, o.zcc).iter().map(|c| game.obj(*c).def.mana_value() as i32).sum()
}

/// `castSpellWithAttributesForFree` over the cards exiled with it (it's been sacrificed: as it last was): a nonland
/// card, chosen among more than one (up to one), "Cast spell without paying its mana cost?", then the cast
fn cast_exiled(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let options: Vec<ObjId> = exiled_with(game, cx.source.0, cx.source.1)
        .into_iter()
        .filter(|c| !game.obj(*c).is(CardType::Land))
        .collect();
    let card = match options.len() {
        0 => return Some(()),
        1 => options[0],
        _ => {
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, you, &options, &options, 0, 1, &rule)?.first() {
                Some(c) => *c,
                None => return Some(()),
            }
        }
    };
    let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
    if choose_use(game, d, you, &message)? {
        cast_free(game, d, you, card);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
