//! Sephiroth, Fabled SOLDIER // Sephiroth, One-Winged Angel: "Whenever Sephiroth enters or attacks, you may sacrifice
//! another creature. If you do, draw a card. Whenever another creature dies, target opponent loses 1 life and you gain
//! 1 life. If this is the fourth time this ability has resolved this turn, transform Sephiroth." // "Flying. Super
//! Nova — As this creature transforms into Sephiroth, One-Winged Angel, you get an emblem with "Whenever a creature
//! dies, target opponent loses 1 life and you gain 1 life." Whenever Sephiroth attacks, you may sacrifice any number
//! of other creatures. If you do, draw that many cards."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "SephirothFabledSOLDIER.SephirothOneWingedAngelEmblemEffect",
        &[EventType::Transforming],
        applies,
        replace,
    )
    .effects(&["new GetEmblemEffect(new SephirothOneWingedAngelEmblem())"]),
    Handler::effect("SephirothFabledSOLDIER.SephirothOneWingedAngelSacrificeEffect", sacrifice_any),
];

/// `TRANSFORMING` of this permanent, still on the battlefield
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target == Some(cx.source) && game.obj(cx.source).zone == Zone::Battlefield
}

/// `game.addEmblem(new SephirothOneWingedAngelEmblem(), obj, source)`; the event goes on
fn replace(game: &mut Game, cx: &StaticCx, _ev: &mut Event) -> bool {
    let Some(EffectDef::GetEmblemEffect { emblem }) = cx.effects.first() else {
        return false;
    };
    token_image(game, emblem, cx.source);
    let id = game.create_obj(Arc::new(emblem.card()), cx.controller, Zone::Library);
    game.obj_mut(id).trigger_order = (1 << 32) + id.0 as u64;
    if let Some((_, _, set, pick)) = game.token_made.take() {
        game.obj_mut(id).token_image = Some((String::new(), set, pick));
    }
    game.move_to(id, Zone::Command);
    false
}

/// `TargetSacrifice(0, MAX, another creature)` chosen with `Player.choose`, each sacrificed in the order chosen, then a
/// card drawn for each one sacrificed (none: nothing)
fn sacrifice_any(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let can: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| *p != src && game.obj(*p).controller == you && game.has_type(*p, CardType::Creature))
        .collect();
    let chosen = choose_cards(game, d, you, &can, &can, 0, usize::MAX, "sacrifice")?;
    let mut n = 0;
    for p in chosen {
        if game.obj(p).zone == Zone::Battlefield && game.obj(p).controller == you {
            sacrifice(game, p);
            n += 1;
        }
    }
    if n > 0 {
        draw(game, you, n);
    }
    Some(())
}
