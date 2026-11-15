//! Borborygmos and Fblthp: "Whenever Borborygmos and Fblthp enters the battlefield or attacks, draw a card, then you
//! may discard any number of land cards. When you discard one or more cards this way, Borborygmos and Fblthp deals
//! twice that much damage to target creature." and "{1}{U}: Put Borborygmos and Fblthp into its owner's library
//! third from the top."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("BorborygmosAndFblthp.BorborygmosAndFblthpDiscardEffect", discard_lands)
        .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"]),
    Handler::effect("BorborygmosAndFblthp.BorborygmosAndFblthpTuckEffect", tuck)
        .effects(&["new PutIntoLibraryNFromTopTargetEffect(3)"]),
];

/// A card drawn; any number of land cards from your hand (`TargetDiscard(0, MAX, lands)` chosen with `choose`)
/// discarded together; then, if any were, `fireReflexiveTriggeredAbility`: twice that much damage to its own target
fn discard_lands(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 1);
    let hand = game.player(you).hand.clone();
    let lands: Vec<ObjId> = hand.iter().copied().filter(|c| game.has_type(*c, CardType::Land)).collect();
    let chosen = choose_cards(game, d, you, &hand, &lands, 0, usize::MAX, "discard")?;
    let mut n = 0;
    for c in chosen {
        if game.obj(c).zone == Zone::Hand && game.obj(c).owner == you {
            discard_card(game, you, c);
            n += 1;
        }
    }
    game.discarded_cards();
    if n == 0 {
        return Some(());
    }
    let target = TargetDef::TargetPermanent {
        filter: cx.filters[0].clone(),
        min: 1,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "{this} deals twice that much damage to target creature.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(2 * n) }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// The source, if it's still the permanent that activated this, third from the top of its owner's library
fn tuck(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.source_on_battlefield(game) {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(cx.source_id())]);
    }
    Some(())
}
