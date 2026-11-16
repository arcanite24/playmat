//! Dragon's Disciple: "As Dragon's Disciple enters, you may reveal a Dragon card from your hand. If you do, or if you
//! control a Dragon, Dragon's Disciple enters with a +1/+1 counter on it. Dragons you control have ward {1}."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DragonsDisciple.DragonsDiscipleEffect", apply)];

/// With a Dragon card in hand: "Reveal a Dragon card from your hand?", then `chooseTarget(TargetCardInHand(Dragon))`
/// and the reveal; a card revealed, or else a Dragon you control, puts the +1/+1 counter on it as it enters
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let hand = game.player(you).hand.clone();
    let dragons: Vec<ObjId> = hand.iter().copied().filter(|c| game.has_subtype(*c, "Dragon")).collect();
    if !dragons.is_empty() && choose_use(game, d, you, "Reveal a Dragon card from your hand?")? {
        let rule = item_rule(game, cx.item);
        if let Some(&c) = choose_cards(game, d, you, &hand, &dragons, 1, 1, &rule)?.first() {
            let zcc = game.obj(me).zcc;
            game.revealed.push(crate::state::Revealed { source: me, zcc, paren: false, cards: vec![(c, Zone::Hand)] });
            add_counters_by(game, me, "+1/+1", 1, Some(you));
            return Some(());
        }
    }
    let dragon = game
        .battlefield
        .iter()
        .any(|o| *o != me && game.obj(*o).controller == you && game.has_subtype(*o, "Dragon"));
    if dragon {
        add_counters_by(game, me, "+1/+1", 1, Some(you));
    }
    Some(())
}
