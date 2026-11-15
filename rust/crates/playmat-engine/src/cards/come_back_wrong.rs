//! Come Back Wrong: "Destroy target creature. If a creature card is put into a graveyard this way, return it to the
//! battlefield under your control. Sacrifice it at the beginning of your next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ComeBackWrong.ComeBackWrongEffect", apply)];

/// The creature destroyed; its card (a mutated one's merged cards too) now in a graveyard and still a creature card
/// comes back under your control, each with a delayed "sacrifice it" at your next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(creature) = cx.obj(0) else { return Some(()) };
    let you = cx.controller();
    let mut left = vec![creature];
    left.extend(game.obj(creature).merged.iter().copied());
    destroy(game, creature);
    let cards: Vec<ObjId> = left
        .into_iter()
        .filter(|c| {
            let o = game.obj(*c);
            o.zone == Zone::Graveyard && !o.token && o.is(CardType::Creature)
        })
        .collect();
    if cards.is_empty() {
        return Some(());
    }
    if !put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield) {
        return None;
    }
    for c in cards {
        if game.obj(c).zone != Zone::Battlefield {
            continue;
        }
        let delayed = crate::state::Delayed {
            rule: "At the beginning of your next end step, sacrifice it.".into(),
            sacrifice: vec![(c, game.obj(c).zcc)],
            warp_exile: Vec::new(),
            effects: Vec::new(),
            source_zcc: cx.source.1,
            fixed: Vec::new(),
            return_tapped: Vec::new(),
            targets: Vec::new(),
            condition: None,
        };
        game.at_your_next_end_step
            .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    }
    Some(())
}
