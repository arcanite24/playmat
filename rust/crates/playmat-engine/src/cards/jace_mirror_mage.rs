//! Jace, Mirror Mage: "When Jace enters, if Jace was kicked, create a token that's a copy of Jace, except it's not
//! legendary and its starting loyalty is 1." and "0: Draw a card and reveal it. Remove a number of loyalty counters
//! equal to that card's mana value from Jace."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("JaceMirrorMage.JaceMirrorMageCopyEffect", copy),
    Handler::effect("JaceMirrorMage.JaceMirrorMageDrawEffect", draw_reveal),
];

/// `CreateTokenCopyTargetEffect` of Jace as it is, or as it last was on the battlefield (`getPermanentOrLKIBattlefield`)
fn copy(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield && game.obj(src).zone != Zone::Graveyard && game.obj(src).zone != Zone::Exile {
        return Some(());
    }
    token_copy_modified(game, d, src, cx.controller(), 1, &|def: &mut CardDef| {
        def.supertypes.retain(|s| s != "LEGENDARY");
        def.loyalty = Some(1);
    });
    if d.stopped() {
        return None;
    }
    Some(())
}

/// The top card, drawn (none drawn, nothing more), revealed; its mana value in loyalty counters off Jace while it's
/// still the permanent that activated this
fn draw_reveal(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else {
        draw(game, you, 1);
        return Some(());
    };
    draw(game, you, 1);
    if game.obj(card).zone != Zone::Hand {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: vec![(card, Zone::Hand)] });
    let mv = game.obj(card).def.mana_value() as i32;
    if mv == 0 || !cx.source_on_battlefield(game) {
        return Some(());
    }
    let o = game.obj_mut(cx.source_id());
    let n = mv.min(o.counter("loyalty"));
    if n > 0 {
        o.add_counters("loyalty", -n);
        o.counters.retain(|(_, k)| *k > 0);
    }
    Some(())
}
