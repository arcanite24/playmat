//! Conduit of Worlds: "You may play lands from your graveyard. {T}: Choose target nonland permanent card in your
//! graveyard. If you haven't cast a spell this turn, you may cast that card. If you do, you can't cast additional
//! spells this turn. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ConduitOfWorlds.ConduitOfWorldsEffect", apply),
    Handler::rule("ConduitOfWorlds.ConduitOfWorldsCantCastEffect", Rule::CantCast(you_cant_cast)),
];

/// `HaventCastSpellThisTurnCondition`, then "Cast …? (You still pay the costs)" and the card cast from the graveyard
/// (`PlayFromNotOwnHandZone`); cast, you can't cast spells for the rest of the turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if game.watch.spells.iter().any(|s| s.caster == you) {
        return Some(());
    }
    let Some(card) = cx.obj(0) else { return Some(()) };
    let name = game.obj(card).name().to_string();
    if !choose_use(game, d, you, &format!("Cast {name}? (You still pay the costs)"))? {
        return Some(());
    }
    let cast = cast_allowed(game, d, you, card);
    if d.stopped() {
        return None;
    }
    if cast {
        let e: EffectDef = serde_json::from_value(
            serde_json::json!({"class": "CardStatic", "handler": "ConduitOfWorlds.ConduitOfWorldsCantCastEffect"}),
        )
        .ok()?;
        cx.run(game, d, &[e]);
    }
    Some(())
}

/// `CAST_SPELL` by the effect's controller, until end of turn
fn you_cant_cast(_: &Game, cx: &StaticCx, seat: Seat, _: ObjId, _: &CardDef) -> bool {
    seat == cx.controller
}
