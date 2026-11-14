//! Sunset Revelry: "If an opponent has more life than you, you gain 4 life. If an opponent controls more creatures
//! than you, create two 1/1 white Human creature tokens. If an opponent has more cards in hand than you, draw a
//! card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SunsetRevelry.SunsetRevelryEffect", apply)
    .effects(&["new CreateTokenEffect(new HumanToken(), 2)"])];

/// each checked as it comes, after the one before it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    let opponents: Vec<Seat> = (1..n).map(|i| (you + i) % n).filter(|p| !game.player(*p).lost).collect();
    if opponents.iter().any(|p| game.player(*p).life > game.player(you).life) {
        gain_life(game, you, 4);
    }
    let creatures = |game: &Game, p: Seat| {
        game.battlefield.iter().filter(|o| game.obj(**o).controller == p && game.has_type(**o, CardType::Creature)).count()
    };
    if opponents.iter().any(|p| creatures(game, *p) > creatures(game, you)) {
        cx.run(game, d, cx.effects);
        if d.stopped() {
            return None;
        }
    }
    if opponents.iter().any(|p| game.player(*p).hand.len() > game.player(you).hand.len()) {
        draw(game, you, 1);
    }
    Some(())
}
