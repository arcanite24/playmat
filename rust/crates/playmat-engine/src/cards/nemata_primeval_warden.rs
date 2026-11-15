//! Nemata, Primeval Warden: "Reach. If a creature an opponent controls would die, exile it instead. When you do,
//! create a 1/1 green Saproling creature token. {G}, Sacrifice a Saproling: Nemata gets +2/+2 until end of turn.
//! {1}{B}, Sacrifice two Saprolings: Draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "NemataPrimevalWarden.NemataPrimevalWardenEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)
.effects(&["new CreateTokenEffect(new SaprolingToken())"])];

/// `isDiesEvent` of a creature (a token too) whose controller is an opponent of Nemata's controller
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    ev.from == Some(Zone::Battlefield)
        && ev.to == Some(Zone::Graveyard)
        && game.has_type(id, CardType::Creature)
        && game.obj(id).controller != cx.controller
}

/// Exiled instead, and the reflexive trigger making the Saproling, with the text Java gives it
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    let delayed = Delayed {
        rule: "If a creature an opponent controls would die, exile it instead. When you do, create a 1/1 green \
               Saproling creature token."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: game.obj(cx.source).zcc,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    let item = StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source, cx.controller) };
    // (fired at once, before the dies triggers of a state-based batch: `fireReflexiveTriggeredAbility`)
    match game.simultaneous_from {
        Some(k) if k <= game.pending.len() => {
            game.pending.insert(k, item);
            game.simultaneous_from = Some(k + 1);
        }
        _ => game.pending.push(item),
    }
    false
}
