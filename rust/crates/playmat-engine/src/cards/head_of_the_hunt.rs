//! Head of the Hunt: "If a creature an opponent controls would die, exile it instead. When you do, create a 2/2 green
//! Wolf creature token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("HeadOfTheHunt.HeadOfTheHuntEffect", &[EventType::ZoneChange], applies, replace)
        .effects(&["new CreateTokenEffect(new WolfToken())"])];

/// `isDiesEvent` of a creature an opponent of yours controls
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    ev.from == Some(Zone::Battlefield)
        && ev.to == Some(Zone::Graveyard)
        && game.has_type(id, CardType::Creature)
        && game.obj(id).controller != cx.controller
}

/// exiled instead, and `fireReflexiveTriggeredAbility`: the Wolf's trigger waits to go on the stack
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    let delayed = Delayed {
        rule: "If a creature an opponent controls would die, exile it instead. When you do, create a 2/2 green Wolf \
               creature token."
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
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source, cx.controller) });
    false
}
