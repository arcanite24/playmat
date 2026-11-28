//! Opening hands: whether the AI keeps or mulligans, and which cards go to the bottom (the London mulligan).
//! Used by the Rust host for its AI seats and by the benches, so both play the same way.

use crate::fm::Fm;
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
/// (`Config::mulligan_net`), by goldfishing (`Config::mulligan_sim`), by Playbook rules when configured, or by counting lands.
pub fn wants(game: &Game, seat: Seat, taken: usize, config: Option<&crate::search::Config>) -> bool {
    if let Some(c) = config {
        if c.mulligan_net && taken < 2 {
            return net_wants(game, seat, taken, c);
        }
        if c.mulligan_sim && taken < 2 {
            return sim_wants(game, seat, taken);
        }
        if let Some(pb) = crate::playbook::resolve(c.playbook.as_deref()) {
            return playbook_wants(game, seat, taken, &pb);
        }
    }
    rule_wants(game, seat, taken)
}

/// The card the AI with `config` puts on the bottom: by the neural value head (`Config::mulligan_net`), by
/// goldfishing (`Config::mulligan_sim`), by Playbook rules when configured, or by the rule.
pub fn bottom(game: &Game, seat: Seat, config: Option<&crate::search::Config>) -> ObjId {
    if let Some(c) = config {
        if c.mulligan_net {
            return net_bottom(game, seat, c);
        }
        if c.mulligan_sim {
            return sim_bottom(game, seat);
        }
        if let Some(pb) = crate::playbook::resolve(c.playbook.as_deref()) {
            return playbook_bottom(game, seat, &pb);
        }
    }
    rule_bottom(game, seat)
}

fn playbook_wants(game: &Game, seat: Seat, taken: usize, pb: &crate::playbook::Playbook) -> bool {
    if taken >= 2 {
        return false;
    }
    let hand = &game.player(seat).hand;
    let parsed = cards(game, hand);
    let lands = parsed.iter().filter(|c| c.land).count();
    let min_l = if taken == 0 { pb.min_lands } else { pb.min_lands.saturating_sub(1).max(1) };
    let max_l = if taken == 0 { pb.max_lands } else { pb.max_lands.min(5) };
    if lands < min_l || lands > max_l {
        return true;
    }
    // On 7 cards, also check color match and early curve action when configured
    if taken == 0 {
        let land_colors = parsed.iter().filter(|c| c.land).fold(0u8, |acc, c| acc | c.colors);
        if pb.require_color_match && land_colors != 0 {
            let castable_any =
                parsed.iter().any(|c| !c.land && c.mv <= (lands as u32 + 1) && (c.colors & !land_colors == 0));
            if !castable_any {
                return true;
            }
        }
        if let Some(max_mv) = pb.require_early_action_by_turn {
            let has_early = parsed.iter().any(|c| !c.land && c.mv <= max_mv);
            if !has_early {
                return true;
            }
        }
    }
    false
}

fn playbook_bottom(game: &Game, seat: Seat, pb: &crate::playbook::Playbook) -> ObjId {
    let hand = &game.player(seat).hand;
    let lands: Vec<ObjId> = hand.iter().copied().filter(|id| game.obj(*id).is(CardType::Land)).collect();
    // Bottom excess lands above the playbook's preferred band
    if lands.len() > pb.max_lands.saturating_sub(1).max(2) {
        return lands[0];
    }
    // Otherwise bottom the highest-MV non-preferred-role spell
    hand.iter()
        .copied()
        .filter(|id| !game.obj(*id).is(CardType::Land))
        .max_by_key(|id| {
            let o = game.obj(*id);
            let preferred = pb.has_any_role(&o.def.name, &pb.prefer_keep_roles);
            (!preferred, o.def.mana_value())
        })
        .unwrap_or(hand[0])
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

fn net_hand_obs(game: &Game, seat: Seat) -> crate::obs::Obs {
    let mut g = game.clone();
    g.turn = 1;
    g.step = Some(playmat_engine::state::Step::PrecombatMain);
    g.priority = Some(seat);
    let playable = rules::playable(&g, seat);
    let q = playmat_engine::decide::Query::Priority { playable };
    crate::obs::observe(&g, seat, &q, &[])
}

#[inline]
fn val_from_logit(logit: f32) -> f64 {
    1.0 / (1.0 + (-(logit as f64)).sexp())
}

#[allow(dead_code)]
fn net_hand_value(net: &crate::net::Net, game: &Game, seat: Seat) -> f64 {
    let obs = net_hand_obs(game, seat);
    val_from_logit(net.forward_batch(&[obs])[0].value)
}

fn net_bottom_with(net: &crate::net::Net, game: &Game, seat: Seat) -> ObjId {
    let hand = game.player(seat).hand.clone();
    if hand.len() <= 1 {
        return hand.first().copied().unwrap_or_else(|| rule_bottom(game, seat));
    }
    let mut obs_batch = Vec::with_capacity(hand.len());
    for &id in &hand {
        let mut g = game.clone();
        put_on_bottom(&mut g, seat, id);
        obs_batch.push(net_hand_obs(&g, seat));
    }
    let outs = net.forward_batch(&obs_batch);
    let mut best: Option<(ObjId, f64)> = None;
    for (&id, out) in hand.iter().zip(outs.iter()) {
        let v = val_from_logit(out.value);
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
    let hand = &game.player(seat).hand;
    let lands = hand.iter().filter(|id| game.obj(**id).is(CardType::Land)).count();
    if taken == 0 && hand.len() == 7 && (lands == 0 || lands == 7) {
        return true;
    }
    if taken == 1 && hand.len() == 6 && (lands == 0 || lands == 6) {
        return true;
    }
    let net = crate::net::of(&cfg.net);
    let mut pool: Vec<ObjId> = hand.iter().chain(game.player(seat).library.iter()).copied().collect();
    let size = 7usize.saturating_sub(taken + 1);
    let mut mix = Mix(seed(game, seat) ^ 0x4E45_544D);
    let deals = 8usize;
    let mut obs_batch = Vec::with_capacity(1 + deals * 7);
    obs_batch.push(net_hand_obs(game, seat));
    let mut deal_ranges: Vec<(usize, usize)> = Vec::with_capacity(deals);
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
        let start = obs_batch.len();
        if g.player(seat).hand.len() > size {
            let cand_hand = g.player(seat).hand.clone();
            if cand_hand.len() <= 1 {
                let c = cand_hand.first().copied().unwrap_or_else(|| rule_bottom(&g, seat));
                put_on_bottom(&mut g, seat, c);
                obs_batch.push(net_hand_obs(&g, seat));
            } else {
                for &id in &cand_hand {
                    let mut gc = g.clone();
                    put_on_bottom(&mut gc, seat, id);
                    obs_batch.push(net_hand_obs(&gc, seat));
                }
            }
        } else {
            obs_batch.push(net_hand_obs(&g, seat));
        }
        deal_ranges.push((start, obs_batch.len()));
    }
    let outs = net.forward_batch(&obs_batch);
    let keep = val_from_logit(outs[0].value);
    let mut mull = 0.0;
    for (start, end) in deal_ranges {
        let mut best_v = f64::NEG_INFINITY;
        for out in &outs[start..end] {
            let v = val_from_logit(out.value);
            if v > best_v {
                best_v = v;
            }
        }
        mull += best_v;
    }
    keep < mull / deals as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duel::{new_duel, Deck};
    use crate::search::Config;

    #[test]
    fn mulligan_zero_seven_land_floor_priority_obs_and_batched_parity() {
        let db = playmat_engine::cards();
        let all_spells = Deck::parse("no_lands", "60 Grizzly Bears\n");
        let all_lands = Deck::parse("only_lands", "60 Forest\n");
        let mixed = Deck::parse("mixed", "24 Forest\n18 Llanowar Elves\n18 Grizzly Bears\n");

        let cfg = Config { mulligan_net: true, ..Config::default() };

        // 0-land 7-card hand MUST always mulligan regardless of neural value noise
        let g_no_land = new_duel(&db, [&all_spells, &mixed], 7, 0).expect("no_lands duel");
        assert!(net_wants(&g_no_land, 0, 0, &cfg), "0-land 7-card hand must mulligan");

        // 7-land 7-card hand MUST always mulligan regardless of neural value noise
        let g_seven_land = new_duel(&db, [&all_lands, &mixed], 7, 0).expect("only_lands duel");
        assert!(net_wants(&g_seven_land, 0, 0, &cfg), "7-land 7-card hand must mulligan");

        // Verify play vs draw observation flags: priority == Some(seat) (glob[32] == 1.0) on BOTH seats!
        let g_mix = new_duel(&db, [&mixed, &mixed], 123, 0).expect("mixed duel");
        let obs_play = net_hand_obs(&g_mix, 0);
        let obs_draw = net_hand_obs(&g_mix, 1);
        assert_eq!(obs_play.glob[31], 1.0, "play seat must have active == 1.0");
        assert_eq!(obs_play.glob[32], 1.0, "play seat must have priority == 1.0");
        assert_eq!(obs_draw.glob[31], 0.0, "draw seat must have active == 0.0");
        assert_eq!(obs_draw.glob[32], 1.0, "draw seat must have priority == 1.0");

        // Playable lands in play seat's hand should be marked playable (f[47] == 1.0)
        let n_ents = obs_play.n_ents();
        let any_playable = (0..n_ents).any(|i| obs_play.ents[i * crate::obs::ENT_F + 47] == 1.0);
        assert!(any_playable, "play seat hand with lands at PrecombatMain must mark lands playable");

        // Batched net_bottom_with & net_wants parity against sequential single-Obs evaluation
        let net = crate::net::of(&cfg.net);
        for seed_i in 1..=6 {
            let g = new_duel(&db, [&mixed, &mixed], 1000 + seed_i, 0).expect("duel");
            let lands = g.player(0).hand.iter().filter(|id| g.obj(**id).is(CardType::Land)).count();
            if lands == 0 || lands == 7 {
                continue;
            }
            let batched_bot = net_bottom_with(&net, &g, 0);
            let mut seq_best: Option<(ObjId, f64)> = None;
            for &id in &g.player(0).hand {
                let mut gc = g.clone();
                put_on_bottom(&mut gc, 0, id);
                let v = val_from_logit(net.forward(&net_hand_obs(&gc, 0)).value);
                if seq_best.is_none_or(|(_, b)| v > b) {
                    seq_best = Some((id, v));
                }
            }
            let seq_bot = seq_best.unwrap().0;
            assert_eq!(batched_bot, seq_bot, "batched net_bottom_with must match sequential argmax");

            // Check net_wants batched vs sequential
            let keep_seq = val_from_logit(net.forward(&net_hand_obs(&g, 0)).value);
            let mut pool: Vec<ObjId> =
                g.player(0).hand.iter().chain(g.player(0).library.iter()).copied().collect();
            let mut mix = Mix(seed(&g, 0) ^ 0x4E45_544D);
            let mut mull_seq = 0.0;
            for _ in 0..8 {
                mix.shuffle(&mut pool);
                let mut gd = g.clone();
                let new_h = pool[..7].to_vec();
                let new_l = pool[7..].to_vec();
                for &id in &new_h {
                    gd.obj_mut(id).zone = Zone::Hand;
                }
                for &id in &new_l {
                    gd.obj_mut(id).zone = Zone::Library;
                }
                gd.player_mut(0).hand = new_h;
                gd.player_mut(0).library = new_l;
                let c = net_bottom_with(&net, &gd, 0);
                put_on_bottom(&mut gd, 0, c);
                mull_seq += val_from_logit(net.forward(&net_hand_obs(&gd, 0)).value);
            }
            let seq_wants = keep_seq < mull_seq / 8.0;
            assert_eq!(net_wants(&g, 0, 0, &cfg), seq_wants, "batched net_wants must match sequential");
        }
    }
}
