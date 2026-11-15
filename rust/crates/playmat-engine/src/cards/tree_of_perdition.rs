//! Tree of Perdition: "Defender. {T}: Exchange target opponent's life total with Tree of Perdition's toughness."

use super::*;
use crate::card::Amount;

pub const HANDLERS: &[Handler] = &[Handler::effect("TreeOfPerdition.TreeOfPerditionEffect", apply)];

/// Nothing unless the Tree is still a creature on the battlefield and the totals differ (nor when the opponent can't
/// gain the life it would); the opponent's life becomes its toughness (`setLife`: a gain or a loss), then
/// `SetBaseToughnessSourceEffect(life, Duration.Custom)` for as long as it stays
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let me = cx.source_id();
    if !cx.source_on_battlefield(game) || !game.has_type(me, CardType::Creature) {
        return Some(());
    }
    let amount = game.power_toughness(me).1;
    let life = game.player(opponent).life;
    if life == amount || (life < amount && game.player(opponent).cant_gain_life) {
        return Some(());
    }
    if amount > life {
        gain_life(game, opponent, amount - life);
    } else {
        lose_life(game, opponent, life - amount);
    }
    let at = game.timestamp();
    let b = crate::state::BasePtSet {
        id: me,
        zcc: cx.source.1,
        power: None,
        toughness: Some(Amount::Fixed(life)),
        source: me,
        controller: cx.controller(),
        x: cx.item.x,
        at,
        until_eot: false,
    };
    game.base_pt_sets.push(b);
    Some(())
}
