//! King Narfi's Betrayal: "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! I — Each player mills four cards. Then you may exile a creature or planeswalker card from each graveyard. II, III —
//! Until end of turn, you may cast spells from among cards exiled with King Narfi's Betrayal, and you may spend mana
//! as though it were mana of any color to cast those spells."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("KingNarfisBetrayal.KingNarfisBetrayalFirstEffect", mill_and_exile)
        .effects(&["new MillCardsEachPlayerEffect(4, TargetController.EACH_PLAYER)"]),
    Handler::effect("KingNarfisBetrayal.KingNarfisBetrayalSecondEffect", castable),
];

fn creature_or_planeswalker(game: &Game, c: ObjId) -> bool {
    game.obj(c).is(CardType::Creature) || game.obj(c).is(CardType::Planeswalker)
}

/// Everyone mills four; then your graveyard and each opponent's in turn, each with a creature or planeswalker card:
/// "Exile a creature or planeswalker card from …?", one chosen (not targeted) and exiled with the Saga
fn mill_and_exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let n = game.seats();
    let order: Vec<Seat> =
        std::iter::once(you).chain((1..n).map(|k| (you + k) % n).filter(|p| !game.player(*p).lost)).collect();
    for p in order {
        let yard = game.player(p).graveyard.clone();
        let possible: Vec<ObjId> = yard.iter().copied().filter(|c| creature_or_planeswalker(game, *c)).collect();
        if possible.is_empty() {
            continue;
        }
        let message = if p == you {
            "Exile a creature or planeswalker card from your graveyard?".to_string()
        } else {
            format!("Exile a creature or planeswalker card from {}'s graveyard?", log::seat_name(p))
        };
        if !choose_use(game, d, you, &message)? {
            continue;
        }
        let rule = "creature or planeswalker card from each graveyard";
        let Some(&c) = choose_cards(game, d, you, &yard, &possible, 1, 1, rule)?.first() else { continue };
        game.move_to(c, Zone::Exile);
        if game.obj(c).zone == Zone::Exile {
            game.obj_mut(c).exiled_with = Some(cx.source);
        }
    }
    Some(())
}

/// Each card still in the Saga's exile zone: castable by you until end of turn, with mana of any type
fn castable(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let turn = game.turn;
    let cards: Vec<ObjId> = game.exile.iter().copied().filter(|c| game.obj(*c).exiled_with == Some(cx.source)).collect();
    for c in cards {
        let o = game.obj_mut(c);
        o.may_play = Some((you, turn));
        o.may_play_end_step = false;
        o.may_play_while = None;
        o.cast_only = true;
        o.any_mana = true;
    }
    Some(())
}
