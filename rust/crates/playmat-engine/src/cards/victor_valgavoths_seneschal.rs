//! Victor, Valgavoth's Seneschal: "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a
//! Room, surveil 2 if this is the first time this ability has resolved this turn. If it's the second time, each
//! opponent discards a card. If it's the third time, put a creature card from a graveyard onto the battlefield under
//! your control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("VictorValgavothsSeneschal.VictorValgavothsSeneschalEffect", apply)];

/// The third time this ability resolved this turn (`AbilityResolvedWatcher`, as `IfAbilityHasResolvedXTimesEffect`
/// counts): a creature card in any graveyard, chosen (not targeted; nothing asked with none), onto the battlefield
/// under your control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((index, _)) = cx.item.ability else { return Some(()) };
    let (turn, now) = (game.turn, game.obj(cx.item.card).zcc);
    let times = game
        .abilities_resolved
        .iter()
        .filter(|r| r.0 == cx.item.card && r.1 == index && r.2 == now && r.3 == turn)
        .count();
    if times != 3 {
        return Some(());
    }
    let you = cx.controller();
    let options: Vec<ObjId> = (0..game.seats())
        .flat_map(|p| game.player(p).graveyard.clone())
        .filter(|c| game.has_type(*c, CardType::Creature))
        .collect();
    if options.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &options, &options, 1, 1, &rule)?.first() else { return Some(()) };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
