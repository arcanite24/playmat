//! Fortune, Loyal Steed: "When Fortune enters, scry 2. Whenever Fortune attacks while saddled, at end of combat,
//! exile it and up to one creature that saddled it this turn, then return those cards to the battlefield under
//! their owner's control. Saddle 1"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FortuneLoyalSteed.FortuneLoyalSteedEffect", flicker)];

/// Fortune if it's still the permanent, and up to one creature that saddled it this turn (`player.choose` of a
/// `TargetPermanent(0, 1, filter, true)`, `SaddledSourceThisTurnPredicate`), exiled and returned together
/// (`ExileThenReturnTargetEffect(false, false)` with fixed targets). The saddlers are read from the saddle cost's
/// marks on them (`SaddledMountWatcher`, by the mount as it was when the trigger was made), so they're found after
/// Fortune has left; the trigger only exists because Fortune attacked saddled.
fn flicker(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (mount, zcc) = cx.source;
    let mark = format!("saddled mount {} {} {}", mount.0, zcc, game.turn);
    let saddlers: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Creature) && game.obj(*o).marks.contains(&mark))
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &saddlers, &saddlers, 0, 1, &rule)?;
    let mut targets = Vec::new();
    if cx.source_on_battlefield(game) {
        targets.push(Target::Obj(mount));
    }
    targets.extend(chosen.into_iter().map(Target::Obj));
    if !targets.is_empty() {
        cx.run_on(game, d, &[EffectDef::ExileThenReturnTargetEffect], targets);
    }
    Some(())
}
