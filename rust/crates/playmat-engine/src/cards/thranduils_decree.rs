//! Thranduil's Decree: "Counter target spell. If a permanent spell is countered this way, exile it instead of putting
//! it into its owner's graveyard. You may cast that card without paying its mana cost for as long as it remains
//! exiled."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThranduilsDecree.ThranduilsDecreeEffect", apply)];

/// A permanent spell (as it is on the stack) is countered into exile (`counter(…, PutCards.EXILED)`) and you may
/// cast it from there without paying its mana cost while it stays (`PlayFromNotOwnHandZoneTargetEffect(EXILED, YOU,
/// Custom, true)`); any other spell is countered as usual
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let spell = cx.obj(0)?;
    if game.obj(spell).zone != Zone::Stack {
        return Some(());
    }
    if !game.obj(spell).def.is_permanent_card() {
        cx.run(game, d, &[EffectDef::CounterTargetEffect]);
        return (!d.stopped()).then_some(());
    }
    cx.run(game, d, &[EffectDef::CounterTargetExileEffect]);
    if d.stopped() {
        return None;
    }
    if game.obj(spell).zone == Zone::Exile {
        let o = game.obj_mut(spell);
        o.may_play = Some((you, u32::MAX));
        o.may_play_end_step = false;
        o.may_play_while = None;
        o.play_free = true;
    }
    Some(())
}
