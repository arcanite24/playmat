//! Multiple Choice: "If X is 1, scry 1, then draw a card. If X is 2, you may choose a player. They return a creature
//! they control to its owner's hand. If X is 3, create a 4/4 blue and red Elemental creature token. If X is 4 or
//! more, do all of the above."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MultipleChoice.MultipleChoiceEffect", apply)
    .tokens(&["Elemental44Token"])
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])];

/// Java's order: scry 1 and draw (X 1 or 4+), the token (X 3 or 4+), then (X 2 or 4+) `TargetPlayer(0, 1, true)`
/// with `Player.choose`, and that player's `TargetControlledCreaturePermanent` (not targeted) chosen by them,
/// returned to its owner's hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x;
    let rule = item_rule(game, cx.item);
    if x == 1 || x >= 4 {
        scry(game, d, you, 1, &rule).then_some(())?;
        draw(game, you, 1);
    }
    if x == 3 || x >= 4 {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    }
    if x != 2 && x < 4 {
        return Some(());
    }
    let players: Vec<Seat> = (0..game.seats()).filter(|p| !game.player(*p).lost).collect();
    let player = loop {
        match ask(game, d, you, &Query::Target { players: players.clone(), objs: Vec::new(), required: false }) {
            Answer::Player(p) if players.contains(&p) => break p,
            Answer::Stop => return None,
            Answer::Player(_) | Answer::Obj(_) => {}
            _ => return Some(()),
        }
        if d.stopped() {
            return None;
        }
    };
    let creatures: Vec<ObjId> = matching(game, &cx.filters[0], player, cx.source_id())
        .into_iter()
        .filter(|o| game.obj(*o).controller == player)
        .collect();
    if creatures.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, player, &creatures, &creatures, 1, 1, &rule)?;
    if let Some(&o) = chosen.first() {
        if game.obj(o).zone == Zone::Battlefield {
            game.move_to(o, Zone::Hand);
        }
    }
    Some(())
}
