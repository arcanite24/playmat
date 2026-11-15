//! Tilonalli's Summoner: "Ascend. Whenever Tilonalli's Summoner attacks, you may pay {X}{R}. If you do, create X 1/1
//! red Elemental creature tokens that are tapped and attacking. At the beginning of the next end step, exile those
//! tokens unless you have the city's blessing."

use super::*;
use crate::card::{Condition, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::effect("TilonallisSummoner.TilonallisSummonerEffect", apply).tokens(&["RedElementalToken"])];

/// "Pay {X}{R}? …", then `announceX(0, MAX)` and the cost paid; paid, X tokens tapped and attacking and one delayed
/// "at the beginning of the next end step" exile of them, unless you have the city's blessing then
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(
        game,
        d,
        you,
        "Pay {X}{R}? If you do, you create X 1/1 red Elemental creature tokens that are tapped and attacking.",
    )? {
        return Some(());
    }
    let x = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: i32::MAX }) {
            Answer::Int(n) if n >= 0 => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let mut cost = ManaCost::default();
    cost.generic = x as u32;
    cost.colored[3] = 1;
    let before = game.clone();
    if !pay_mana(game, d, you, &cost) {
        if d.stopped() {
            return None;
        }
        *game = before;
        return Some(());
    }
    let first = game.objs.len();
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: crate::card::Amount::Fixed(x),
        tapped_and_attacking: true,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    };
    cx.run(game, d, &[e]);
    let made: Vec<(ObjId, u32)> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    let delayed = Delayed {
        rule: "At the beginning of the next end step, exile those tokens unless you have the city's blessing.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed: made,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: Some(Condition::Not { x: Box::new(Condition::CitysBlessing) }),
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
