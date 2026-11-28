//! Torrent Sculptor // Flamethrower Sonata. Torrent Sculptor: "When Torrent Sculptor enters, exile an instant or
//! sorcery card from your graveyard. Put a number of +1/+1 counters on Torrent Sculptor equal to half that card's
//! mana value, rounded up." Flamethrower Sonata: "Discard a card, then draw a card. When you discard an instant or
//! sorcery card this way, Flamethrower Sonata deals damage equal to that card's mana value to target creature or
//! planeswalker you don't control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TorrentSculptor.TorrentSculptorEffect", sculpt),
    Handler::effect("TorrentSculptor.FlamethrowerSonataEffect", sonata).filters(&["filter"]),
];

fn instant_or_sorcery(game: &Game, c: ObjId) -> bool {
    game.has_type(c, CardType::Instant) || game.has_type(c, CardType::Sorcery)
}

/// With an instant or sorcery card in your graveyard: one chosen (`TargetCardInYourGraveyard`, not targeted), its
/// mana value halved and rounded up, the card exiled, that many +1/+1 counters on the source while it's still there
fn sculpt(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let grave = game.player(you).graveyard.clone();
    let possible: Vec<ObjId> = grave.iter().copied().filter(|c| instant_or_sorcery(game, *c)).collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &grave, &possible, 1, 1, &rule)?.first() else { return Some(()) };
    let mv = game.obj(card).def.mana_value() as i32;
    let n = mv / 2 + mv % 2;
    game.move_to(card, Zone::Exile);
    if cx.source_on_battlefield(game) && n > 0 {
        put_counters(game, cx.source_id(), "+1/+1", n, you);
    }
    Some(())
}

/// `discardOne`, then a draw; an instant or sorcery discarded: the reflexive trigger, damage equal to its mana value
/// to target creature or planeswalker you don't control
fn sonata(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let discarded = discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    draw(game, you, 1);
    let Some(&card) = discarded.first() else { return Some(()) };
    if !instant_or_sorcery(game, card) {
        return Some(());
    }
    let mv = game.obj(card).def.mana_value() as i32;
    let target = TargetDef::TargetPermanent {
        filter: cx.filters.first()?.clone(),
        min: 1,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "{this} deals damage equal to that card's mana value to target creature or planeswalker you don't \
               control."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(mv) }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
