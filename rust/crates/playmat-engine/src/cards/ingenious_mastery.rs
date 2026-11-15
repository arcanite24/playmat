//! Ingenious Mastery: "You may pay {2}{U} rather than pay this spell's mana cost. If the {2}{U} cost was paid, you
//! draw three cards, then an opponent creates two Treasure tokens and they scry 2. If that cost wasn't paid, you
//! draw X cards."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("IngeniousMastery.IngeniousMasteryEffect", apply).tokens(&["TreasureToken"])];

/// X cards; or, cast for its alternative cost, three cards, then an opponent you choose (`chooseTarget` of a
/// not-targeted `TargetOpponent`) creates two Treasures and scries 2
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !game.obj(cx.source_id()).alternative_paid {
        draw(game, you, cx.item.x.max(0) as usize);
        return Some(());
    }
    draw(game, you, 3);
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    if opponents.is_empty() {
        return Some(());
    }
    let opponent = loop {
        match ask(game, d, you, &Query::Target { players: opponents.clone(), objs: Vec::new(), required: true }) {
            Answer::Player(p) if opponents.contains(&p) => break p,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens.first()?.clone(),
        amount: crate::card::Amount::Fixed(2),
        tapped_and_attacking: false,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    };
    cx.run_as(game, d, &[e], opponent);
    if d.stopped() {
        return None;
    }
    let rule = item_rule(game, cx.item);
    scry(game, d, opponent, 2, &rule).then_some(())
}
