//! Rally the Ancestors: "Return each creature card with mana value X or less from your graveyard to the battlefield.
//! Exile those creatures at the beginning of your next upkeep. Exile Rally the Ancestors."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RallyTheAncestors.RallyTheAncestorsEffect", apply)];

/// The creature cards in your graveyard (in its order) with mana value X or less, onto the battlefield together, then
/// `AtTheBeginOfYourNextUpkeepDelayedTriggeredAbility(ExileTargetEffect)` on the ones that arrived (`FixedTargets`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0);
    let cards: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).def.is(CardType::Creature) && game.obj(*c).def.mana_value() as i32 <= x)
        .collect();
    put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield);
    if d.stopped() {
        return None;
    }
    let fixed: Vec<(ObjId, u32)> = cards
        .iter()
        .filter(|c| game.obj(**c).zone == Zone::Battlefield)
        .map(|c| (*c, game.obj(*c).zcc))
        .collect();
    let delayed = Delayed {
        rule: "At the beginning of your next upkeep, Exile those creatures at the beginning of your next upkeep.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_first_upkeep.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source.0, you) });
    Some(())
}
