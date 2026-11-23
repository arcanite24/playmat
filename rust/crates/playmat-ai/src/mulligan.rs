//! Opening hands: whether the AI keeps or mulligans, and which cards go to the bottom (the London mulligan).
//! Used by the Rust host for its AI seats and by the benches, so both play the same way.

use playmat_engine::card::CardType;
use playmat_engine::rules;
use playmat_engine::state::{Game, ObjId, Seat, Zone};

/// Whether the AI mulligans `seat`'s hand after `taken` mulligans that cost a card (free ones don't count): two to
/// five lands are kept on seven, one to five on six, and anything on five or fewer.
pub fn rule_wants(game: &Game, seat: Seat, taken: usize) -> bool {
    let hand = &game.player(seat).hand;
    let lands = hand.iter().filter(|id| game.obj(**id).is(CardType::Land)).count();
    match taken {
        0 => !(2..=5).contains(&lands),
        1 => !(1..=5).contains(&lands),
        _ => false,
    }
}

/// The card the rule puts on the bottom: a land when the hand has more than three, else its most expensive spell.
pub fn rule_bottom(game: &Game, seat: Seat) -> ObjId {
    let hand = &game.player(seat).hand;
    let lands: Vec<ObjId> = hand.iter().copied().filter(|id| game.obj(*id).is(CardType::Land)).collect();
    if lands.len() > 3 {
        return lands[0];
    }
    hand.iter()
        .copied()
        .filter(|id| !game.obj(*id).is(CardType::Land))
        .max_by_key(|id| game.obj(*id).def.mana_value())
        .unwrap_or(hand[0])
}

/// The London mulligan for seats that all decide by `wants` and `bottom` (the benches; the host runs its own loop,
/// with prompts for people): the starting player first, `free` mulligans that keep the hand's size.
pub fn london(
    game: &mut Game,
    first: Seat,
    free: usize,
    wants: &mut dyn FnMut(&Game, Seat, usize) -> bool,
    bottom: &mut dyn FnMut(&Game, Seat) -> ObjId,
) {
    let seats = game.players.len();
    let mut kept = vec![false; seats];
    let mut taken = vec![0usize; seats];
    let size_after = |taken: usize| 7usize.saturating_sub(taken.saturating_sub(free));
    let order: Vec<Seat> = (0..seats).map(|k| (first + k) % seats).collect();
    while kept.iter().any(|k| !k) {
        let mut going = Vec::new();
        for &seat in &order {
            if kept[seat] {
                continue;
            }
            if size_after(taken[seat]) > 0 && wants(game, seat, taken[seat].saturating_sub(free)) {
                going.push(seat);
            } else {
                kept[seat] = true;
            }
        }
        for seat in going {
            taken[seat] += 1;
            redraw(game, seat);
            while game.player(seat).hand.len() > size_after(taken[seat]) {
                let card = bottom(game, seat);
                put_on_bottom(game, seat, card);
            }
        }
    }
}

/// Shuffles the hand into the library and draws seven
pub fn redraw(game: &mut Game, seat: Seat) {
    let hand = std::mem::take(&mut game.player_mut(seat).hand);
    for id in &hand {
        game.obj_mut(*id).zone = Zone::Library;
    }
    let mut library = std::mem::take(&mut game.player_mut(seat).library);
    library.extend(hand);
    game.rng.shuffle(&mut library);
    game.player_mut(seat).library = library;
    rules::draw(game, seat, 7);
}

/// A card from the hand to the bottom of the library
pub fn put_on_bottom(game: &mut Game, seat: Seat, card: ObjId) {
    let p = game.player_mut(seat);
    p.hand.retain(|x| *x != card);
    p.library.push(card);
    game.obj_mut(card).zone = Zone::Library;
}

/// Whether the AI with `config` (`None`: the quick answers alone) mulligans `seat`'s hand: by the neural value head
/// (`Config::mulligan_net`), by goldfishing (`Config::mulligan_sim`), or by counting lands.
pub fn wants(game: &Game, seat: Seat, taken: usize, config: Option<&crate::search::Config>) -> bool {
    match config {
        Some(c) if c.mulligan_net && taken < 2 => net_wants(game, seat, taken, c),
        Some(c) if c.mulligan_sim && taken < 2 => sim_wants(game, seat, taken),
        _ => rule_wants(game, seat, taken),
    }
}

/// The card the AI with `config` puts on the bottom: by the neural value head (`Config::mulligan_net`), by
/// goldfishing (`Config::mulligan_sim`), or by the rule.
pub fn bottom(game: &Game, seat: Seat, config: Option<&crate::search::Config>) -> ObjId {
    match config {
        Some(c) if c.mulligan_net => net_bottom(game, seat, c),
        Some(c) if c.mulligan_sim => sim_bottom(game, seat),
        _ => rule_bottom(game, seat),
    }
}

/// How many shuffles each goldfish estimate averages
const SHUFFLES: usize = 32;

/// A card as the goldfish sees it
#[derive(Clone, Copy)]
struct Card {
    land: bool,
    /// the colors a land makes, or a spell needs, as bits
    colors: u8,
    mv: u32,
}

fn card(game: &Game, id: ObjId) -> Card {
    let d = &game.obj(id).def;
    if d.types.contains(&CardType::Land) {
        let mut colors = 0u8;
        for a in &d.abilities {
            if let Some(m) = a.mana() {
                for (i, n) in m.colored.iter().enumerate() {
                    if *n > 0 {
                        colors |= 1 << i;
                    }
                }
            }
        }
        Card { land: true, colors, mv: 0 }
    } else {
        let cost = d.printed();
        let colors = cost.colored.iter().enumerate().filter(|(_, n)| **n > 0).fold(0u8, |c, (i, _)| c | 1 << i);
        Card { land: false, colors, mv: d.mana_value() }
    }
}

/// splitmix64: shuffles of the goldfish's own, so estimates don't touch the game's random numbers
struct Mix(u64);
impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

/// The mana a hand spends on turns 1–4 with nothing in the way, drawing from `library` (in order): a land a turn,
/// one that makes a missing color first, then the most expensive spells that fit. Later turns count more.
fn goldfish(hand: &[Card], library: &[Card], on_play: bool) -> f64 {
    let mut hand = hand.to_vec();
    let mut draws = library.iter();
    let (mut lands, mut colors, mut score) = (0u32, 0u8, 0.0);
    for turn in 1..=4u32 {
        if turn > 1 || !on_play {
            if let Some(c) = draws.next() {
                hand.push(*c);
            }
        }
        let needed = hand.iter().filter(|c| !c.land).fold(0u8, |n, c| n | c.colors) & !colors;
        let land =
            hand.iter().position(|c| c.land && c.colors & needed != 0).or_else(|| hand.iter().position(|c| c.land));
        if let Some(i) = land {
            colors |= hand[i].colors;
            lands += 1;
            hand.swap_remove(i);
        }
        let mut mana = lands;
        loop {
            let best = hand
                .iter()
                .enumerate()
                .filter(|(_, c)| !c.land && c.mv <= mana && c.mv > 0 && c.colors & !colors == 0)
                .max_by_key(|(_, c)| c.mv)
                .map(|(i, _)| i);
            let Some(i) = best else { break };
            mana -= hand[i].mv;
            score += (hand[i].mv * turn) as f64;
            hand.swap_remove(i);
        }
    }
    score
}

fn cards(game: &Game, ids: &[ObjId]) -> Vec<Card> {
    ids.iter().map(|id| card(game, *id)).collect()
}

fn seed(game: &Game, seat: Seat) -> u64 {
    game.player(seat).hand.iter().fold(0xC0FFEE ^ seat as u64, |h, id| h.wrapping_mul(31).wrapping_add(id.0 as u64))
}

/// The hand's mean goldfish over shuffles of `library`
fn keep_value(hand: &[Card], library: &[Card], on_play: bool, mix: &mut Mix) -> f64 {
    let mut lib = library.to_vec();
    (0..SHUFFLES)
        .map(|_| {
            mix.shuffle(&mut lib);
            goldfish(hand, &lib, on_play)
        })
        .sum::<f64>()
        / SHUFFLES as f64
}

/// The best hand of `size` from `seven`: the goldfish's best bottom, card by card
fn best_of(seven: &[Card], size: usize, library: &[Card], on_play: bool, mix: &mut Mix) -> Vec<Card> {
    let mut hand = seven.to_vec();
    while hand.len() > size {
        let i = (0..hand.len())
            .map(|i| {
                let mut h = hand.clone();
                h.remove(i);
                (i, keep_value(&h, library, on_play, &mut Mix(mix.next())))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0;
        hand.remove(i);
    }
    hand
}

fn sim_wants(game: &Game, seat: Seat, taken: usize) -> bool {
    let p = game.player(seat);
    let on_play = game.active == Some(seat);
    let hand = cards(game, &p.hand);
    let library = cards(game, &p.library);
    let mut mix = Mix(seed(game, seat));
    let keep = keep_value(&hand, &library, on_play, &mut mix);
    // a mulligan: seven new cards from the hand and library, down to one fewer than now
    let mut all: Vec<Card> = hand.iter().chain(library.iter()).copied().collect();
    let size = 7usize.saturating_sub(taken + 1);
    let deals = 8;
    let mut mull = 0.0;
    for _ in 0..deals {
        mix.shuffle(&mut all);
        let (seven, rest) = all.split_at(7.min(all.len()));
        let h = if size + 1 < seven.len() {
            // cheaper than goldfishing every bottom: the rule's bottoms past the first
            let mut h = seven.to_vec();
            while h.len() > size + 1 {
                h.sort_by_key(|c| if c.land { 0 } else { c.mv });
                h.pop();
            }
            best_of(&h, size, rest, on_play, &mut mix)
        } else {
            best_of(seven, size, rest, on_play, &mut mix)
        };
        mull += keep_value(&h, rest, on_play, &mut mix);
    }
    keep < mull / deals as f64
}

fn sim_bottom(game: &Game, seat: Seat) -> ObjId {
    let p = game.player(seat);
    let on_play = game.active == Some(seat);
    let library = cards(game, &p.library);
    let hand = cards(game, &p.hand);
    let mut mix = Mix(seed(game, seat) ^ 0xB0770);
    let mut best: Option<(ObjId, f64)> = None;
    for (i, id) in p.hand.iter().enumerate() {
        let mut h = hand.clone();
        h.remove(i);
        let v = keep_value(&h, &library, on_play, &mut Mix(mix.next()));
        if best.is_none_or(|(_, b)| v > b) {
            best = Some((*id, v));
        }
    }
    best.map_or_else(|| rule_bottom(game, seat), |(id, _)| id)
}

fn net_hand_value(net: &crate::net::Net, game: &Game, seat: Seat) -> f64 {
    let mut g = game.clone();
    g.turn = 1;
    g.step = Some(playmat_engine::state::Step::PrecombatMain);
    let q = playmat_engine::decide::Query::Priority { playable: Vec::new() };
    let obs = crate::obs::observe(&g, seat, &q, &[]);
    let logit = net.state(&obs).1 as f64;
    1.0 / (1.0 + (-logit).exp())
}

fn net_bottom_with(net: &crate::net::Net, game: &Game, seat: Seat) -> ObjId {
    let hand = game.player(seat).hand.clone();
    if hand.len() <= 1 {
        return hand.first().copied().unwrap_or_else(|| rule_bottom(game, seat));
    }
    let mut best: Option<(ObjId, f64)> = None;
    for id in hand {
        let mut g = game.clone();
        put_on_bottom(&mut g, seat, id);
        let v = net_hand_value(net, &g, seat);
        if best.is_none_or(|(_, b)| v > b) {
            best = Some((id, v));
        }
    }
    best.map_or_else(|| rule_bottom(game, seat), |(id, _)| id)
}

fn net_bottom(game: &Game, seat: Seat, cfg: &crate::search::Config) -> ObjId {
    let net = crate::net::of(&cfg.net);
    net_bottom_with(&net, game, seat)
}

fn net_wants(game: &Game, seat: Seat, taken: usize, cfg: &crate::search::Config) -> bool {
    let net = crate::net::of(&cfg.net);
    let keep = net_hand_value(&net, game, seat);
    let mut pool: Vec<ObjId> = game
        .player(seat)
        .hand
        .iter()
        .chain(game.player(seat).library.iter())
        .copied()
        .collect();
    let size = 7usize.saturating_sub(taken + 1);
    let mut mix = Mix(seed(game, seat) ^ 0x4E45_544D);
    let deals = 8usize;
    let mut mull = 0.0;
    for _ in 0..deals {
        mix.shuffle(&mut pool);
        let mut g = game.clone();
        let draw_n = 7.min(pool.len());
        let new_hand = pool[..draw_n].to_vec();
        let new_lib = pool[draw_n..].to_vec();
        for &id in &new_hand {
            g.obj_mut(id).zone = Zone::Hand;
        }
        for &id in &new_lib {
            g.obj_mut(id).zone = Zone::Library;
        }
        g.player_mut(seat).hand = new_hand;
        g.player_mut(seat).library = new_lib;
        while g.player(seat).hand.len() > size + 1 {
            let c = rule_bottom(&g, seat);
            put_on_bottom(&mut g, seat, c);
        }
        while g.player(seat).hand.len() > size {
            let c = net_bottom_with(&net, &g, seat);
            put_on_bottom(&mut g, seat, c);
        }
        mull += net_hand_value(&net, &g, seat);
    }
    keep < mull / deals as f64
}
