//! Lagomos, Hand of Hatred: "At the beginning of combat on your turn, create a 2/1 red Elemental creature token with
//! trample and haste. Sacrifice it at the beginning of the next end step. {T}: Search your library for a card, put it
//! into your hand, then shuffle. Activate only if five or more creatures died this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LagomosHandOfHatred.LagomosHandOfHatredEffect", apply)
        .effects(&["new CreateTokenEffect(new Elemental21TrampleHasteToken())"]),
    Handler::condition("LagomosHandOfHatred.LagomosHandOfHatredCondition", five_died),
];

/// the token, then a delayed "sacrifice it" for each token made (`getLastAddedTokenIds`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for t in made {
        let delayed = Delayed {
            rule: "At the beginning of the next end step, sacrifice it.".into(),
            sacrifice: vec![(t, game.obj(t).zcc)],
            warp_exile: Vec::new(),
            effects: Vec::new(),
            source_zcc: cx.source.1,
            fixed: Vec::new(),
            return_tapped: Vec::new(),
            targets: Vec::new(),
            condition: None,
        };
        game.at_next_end_step
            .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    }
    Some(())
}

/// `CreaturesDiedWatcher`: five or more creatures died this turn
fn five_died(game: &Game, _you: Seat, _source: ObjId) -> bool {
    game.creatures_died >= 5
}
