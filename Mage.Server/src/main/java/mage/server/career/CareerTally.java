package mage.server.career;

import mage.ObjectColor;
import mage.constants.CommanderCardType;
import mage.constants.SubType;
import mage.constants.WatcherScope;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.events.DamagedEvent;
import mage.game.events.GameEvent;
import mage.game.events.ManaEvent;
import mage.game.events.ZoneChangeEvent;
import mage.game.permanent.Permanent;
import mage.game.stack.Spell;
import mage.players.Player;
import mage.watchers.Watcher;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * What one player did in one game, as named counters, for Career quests, achievements and lifetime stats. Every human
 * seat of a game has its own; one watcher per game fills them all (watchers see every event). Like the game event feed,
 * tallies are found by game id because the watcher is deep-copied with the game state. The AI's simulations run on
 * copies and are skipped.
 * <p>
 * A tally is "tainted" when the game shouldn't count: practice tools or cheats were used, or the opponent was a
 * goldfish. A tainted tally pays no progress.
 */
public final class CareerTally {

    /** tallies older than this are dropped when a new one opens (games that never reported an end) */
    private static final long STALE_MILLIS = 12L * 60 * 60 * 1000;
    /** game id to each human seat's tally, by player id */
    private static final Map<UUID, Map<UUID, CareerTally>> TALLIES = new ConcurrentHashMap<>();

    public static final String SPELLS = "spells";
    public static final String WINS = "wins";
    public static final String GAMES = "games";

    final UUID playerId;
    private final long opened = System.currentTimeMillis();
    private final Map<String, Integer> counters = new TreeMap<>();
    private volatile String taint;
    private int spellsThisTurn;
    private int manaThisTurn;
    private int lowestLife = Integer.MAX_VALUE;
    /** opponents this player dealt the latest damage to, for pod eliminations */
    private final Set<UUID> lastHit = new HashSet<>();
    /** one-game feats (B309): the best value of each and the turn it happened on */
    private final Map<String, Moment> moments = new LinkedHashMap<>();

    /** A one-game feat: what it was, how big, and on which turn. */
    public static final class Moment {
        public final String id;
        public final int value;
        public final int turn;

        Moment(String id, int value, int turn) {
            this.id = id;
            this.value = value;
            this.turn = turn;
        }
    }

    /** the feats a moment needs to reach, by id; below it, a game's best isn't worth telling */
    static final Map<String, Integer> MOMENT_THRESHOLDS = new LinkedHashMap<>();

    static {
        MOMENT_THRESHOLDS.put("big_hit", 10);
        MOMENT_THRESHOLDS.put("storm", 5);
        MOMENT_THRESHOLDS.put("mana_surge", 10);
        MOMENT_THRESHOLDS.put("life_swing", 10);
        MOMENT_THRESHOLDS.put("elimination", 1);
        MOMENT_THRESHOLDS.put("commander_hit", 7);
    }

    private CareerTally(UUID playerId) {
        this.playerId = playerId;
    }

    /** starts tallying a game for one human player; the watcher goes into the game state */
    public static Watcher open(UUID gameId, UUID playerId) {
        return open(gameId, java.util.Collections.singletonList(playerId));
    }

    /** starts tallying a game for each of its human players; one watcher goes into the game state */
    public static Watcher open(UUID gameId, java.util.Collection<UUID> playerIds) {
        long now = System.currentTimeMillis();
        TALLIES.values().removeIf(seats -> seats.values().stream().allMatch(tally -> now - tally.opened > STALE_MILLIS));
        Map<UUID, CareerTally> seats = new java.util.LinkedHashMap<>();
        for (UUID playerId : playerIds) {
            seats.put(playerId, new CareerTally(playerId));
        }
        TALLIES.put(gameId, java.util.Collections.synchronizedMap(seats));
        return new Recorder(gameId);
    }

    /** a seat's tally, or null; it stays until taken */
    public static CareerTally of(UUID gameId, UUID playerId) {
        Map<UUID, CareerTally> seats = gameId == null ? null : TALLIES.get(gameId);
        return seats == null || playerId == null ? null : seats.get(playerId);
    }

    /** the game's only (or first) tally, or null */
    public static CareerTally of(UUID gameId) {
        Map<UUID, CareerTally> seats = gameId == null ? null : TALLIES.get(gameId);
        if (seats == null) {
            return null;
        }
        synchronized (seats) {
            return seats.values().stream().findFirst().orElse(null);
        }
    }

    /** removes and returns every seat's tally of the game, by player id (empty when nothing was tallied) */
    public static Map<UUID, CareerTally> takeAll(UUID gameId) {
        Map<UUID, CareerTally> seats = gameId == null ? null : TALLIES.remove(gameId);
        if (seats == null) {
            return java.util.Collections.emptyMap();
        }
        synchronized (seats) {
            return new java.util.LinkedHashMap<>(seats);
        }
    }

    /** removes the game's tallies and returns the only (or first) one */
    public static CareerTally take(UUID gameId) {
        return takeAll(gameId).values().stream().findFirst().orElse(null);
    }

    /** the game won't count for Career progress for anyone in it, and why */
    public static void taint(UUID gameId, String why) {
        Map<UUID, CareerTally> seats = gameId == null ? null : TALLIES.get(gameId);
        if (seats == null) {
            return;
        }
        synchronized (seats) {
            for (CareerTally tally : seats.values()) {
                if (tally.taint == null) {
                    tally.taint = why;
                }
            }
        }
    }

    /** one seat's game won't count (a player who used help), and why */
    public static void taint(UUID gameId, UUID playerId, String why) {
        CareerTally tally = of(gameId, playerId);
        if (tally != null && tally.taint == null) {
            tally.taint = why;
        }
    }

    public UUID playerId() {
        return playerId;
    }

    /** a tally outside a running game (tests, or a game nothing was recorded for) */
    static CareerTally forTest(UUID playerId) {
        return new CareerTally(playerId);
    }

    public String taint() {
        return taint;
    }

    public synchronized Map<String, Integer> counters() {
        return new TreeMap<>(counters);
    }

    public synchronized int get(String counter) {
        return counters.getOrDefault(counter, 0);
    }

    synchronized void add(String counter, int amount) {
        if (amount != 0) {
            counters.merge(counter, amount, Integer::sum);
        }
    }

    synchronized void max(String counter, int value) {
        counters.merge(counter, value, Math::max);
    }

    synchronized void set(String counter, int value) {
        counters.put(counter, value);
    }

    /** a feat in this game: kept when it's the biggest of its kind so far and big enough to tell */
    synchronized void moment(String id, int value, int turn) {
        Integer threshold = MOMENT_THRESHOLDS.get(id);
        Moment best = moments.get(id);
        if ((threshold == null || value >= threshold) && (best == null || value > best.value)) {
            moments.put(id, new Moment(id, value, turn));
        }
    }

    /** the game's feats, in the order they first happened */
    public synchronized List<Moment> moments() {
        return new ArrayList<>(moments.values());
    }

    /**
     * The game is over: adds the counters only the end can tell (won, the life it ended on, how it was won).
     *
     * @param turns the game's turn number at the end
     */
    /**
     * What kind of game it was (B288, B290): games and wins by kind, format and the deck's colors, for quests,
     * masteries and achievements. Colors are W, U, B, R, G, plus multicolor or colorless.
     */
    public synchronized void context(String kind, String format, java.util.Collection<String> colors, boolean won, boolean bo3MatchWon) {
        java.util.List<String> keys = new java.util.ArrayList<>();
        if (kind != null) {
            keys.add("kind_" + kind);
        }
        if (format != null) {
            keys.add("format_" + format);
        }
        if (colors != null) {
            for (String color : colors) {
                keys.add("color_" + color);
            }
        }
        for (String key : keys) {
            add("games_" + key, 1);
            if (won) {
                add("wins_" + key, 1);
            }
        }
        if (bo3MatchWon) {
            add("bo3_match_wins", 1);
        }
    }

    public synchronized void finish(Game game, boolean won, int turns) {
        add(GAMES, 1);
        add("turns_taken", get("my_turns"));
        if (won) {
            add(WINS, 1);
        }
        if (game == null) {
            return; // nothing was recorded: how the game ended is unknown
        }
        Player me = game.getPlayer(playerId);
        int life = me == null ? 0 : me.getLife();
        int startingLife = game.getStartingLife();
        lowestLife = Math.min(lowestLife, life);
        set("turns", turns);
        set("life_at_end", life);
        if (!won) {
            return;
        }
        if (get("max_attacker_power") >= 5) {
            add("wins_power5", 1);
        }
        if (lowestLife >= 10) {
            add("wins_healthy", 1);
        }
        if (life == 1) {
            set("won_at_one_life", 1);
            moment("one_life", 1, turns);
        }
        if (lowestLife <= 5) {
            set("won_from_five_life", 1);
            moment("comeback", lowestLife, turns);
        }
        if (get("damage_taken") == 0) {
            set("won_flawless", 1);
            moment("flawless", 1, turns);
        }
        if (life > startingLife) {
            set("won_above_starting_life", 1);
        }
        if (get("creature_spells") == 0) {
            set("won_without_creatures", 1);
        }
        // the game's turn counter counts both players' turns; a win by your fourth turn is turn 8 at the latest
        if (get("my_turns") <= 4) {
            set("won_by_turn_four", 1);
            moment("fast_win", get("my_turns"), turns);
        }
        {
            for (UUID opponentId : game.getOpponents(playerId)) {
                Player opponent = game.getPlayer(opponentId);
                if (opponent == null) {
                    continue;
                }
                if (opponent.getCountersCount(mage.counters.CounterType.POISON) >= 10) {
                    set("won_by_poison", 1);
                } else if (opponent.getLibrary().size() == 0 && opponent.getLife() > 0) {
                    set("won_by_mill", 1);
                }
                if (opponent.getLife() <= -10) {
                    set("won_overkill", 1);
                }
            }
        }
    }

    private synchronized void lifeNow(int life) {
        lowestLife = Math.min(lowestLife, life);
    }

    /** the watcher that fills a tally */
    static final class Recorder extends Watcher {

        private final UUID gameId;

        Recorder(UUID gameId) {
            super(WatcherScope.GAME);
            this.gameId = gameId;
        }

        @Override
        public void watch(GameEvent event, Game game) {
            if (game.isSimulation()) {
                return;
            }
            Map<UUID, CareerTally> seats = TALLIES.get(gameId);
            if (seats == null) {
                return;
            }
            CareerTally[] tallies;
            synchronized (seats) {
                tallies = seats.values().toArray(new CareerTally[0]);
            }
            for (CareerTally tally : tallies) {
                watch(tally, event, game);
            }
        }

        private static void watch(CareerTally tally, GameEvent event, Game game) {
            UUID me = tally.playerId;
            switch (event.getType()) {
                case BEGIN_TURN:
                    synchronized (tally) {
                        tally.spellsThisTurn = 0;
                        tally.manaThisTurn = 0;
                    }
                    if (me.equals(game.getActivePlayerId())) {
                        tally.add("my_turns", 1);
                    }
                    break;
                case SPELL_CAST:
                    if (me.equals(event.getPlayerId())) {
                        spell(tally, game.getSpell(event.getTargetId()), game);
                        if (event.getZone() == Zone.GRAVEYARD) {
                            tally.add("cast_from_graveyard", 1);
                        }
                    }
                    break;
                case COUNTERED:
                    // the countering ability's controller counters another player's spell
                    if (me.equals(game.getControllerId(event.getSourceId())) && !me.equals(event.getPlayerId())) {
                        tally.add("spells_countered", 1);
                    }
                    break;
                case ACTIVATED_ABILITY: {
                    Permanent source = game.getPermanent(event.getSourceId());
                    if (me.equals(event.getPlayerId()) && source != null && source.isPlaneswalker(game)) {
                        tally.add("loyalty_abilities", 1);
                    }
                    break;
                }
                case ATTACHED: {
                    Permanent attachment = game.getPermanent(event.getSourceId());
                    if (me.equals(event.getPlayerId()) && attachment != null) {
                        if (attachment.hasSubtype(SubType.AURA, game)) {
                            tally.add("auras_attached", 1);
                        } else if (attachment.hasSubtype(SubType.EQUIPMENT, game)) {
                            tally.add("equipment_attached", 1);
                        }
                    }
                    break;
                }
                case SACRIFICED_PERMANENT:
                    if (me.equals(event.getPlayerId())) {
                        tally.add("sacrificed", 1);
                    }
                    break;
                case DISCARDED_CARD:
                    if (event.getPlayerId() != null && !me.equals(event.getPlayerId())) {
                        tally.add("opponent_discards", 1);
                    }
                    break;
                case MILLED_CARD:
                    if (event.getPlayerId() != null && !me.equals(event.getPlayerId())) {
                        tally.add("opponent_milled", 1);
                    }
                    break;
                case MANA_ADDED:
                    if (me.equals(event.getPlayerId()) && event instanceof ManaEvent && ((ManaEvent) event).getMana() != null) {
                        int mana;
                        synchronized (tally) {
                            tally.manaThisTurn += ((ManaEvent) event).getMana().count();
                            mana = tally.manaThisTurn;
                        }
                        tally.max("most_mana_in_a_turn", mana);
                        tally.moment("mana_surge", mana, game.getTurnNum());
                    }
                    break;
                case LOST:
                    if (!me.equals(event.getPlayerId()) && game.getPlayers().size() > 2) {
                        boolean mine;
                        synchronized (tally) {
                            mine = tally.lastHit.contains(event.getPlayerId());
                        }
                        if (mine) {
                            tally.add("pod_eliminations", 1);
                            tally.moment("elimination", tally.get("pod_eliminations"), game.getTurnNum());
                        }
                    }
                    break;
                case DAMAGED_PLAYER: {
                    boolean combat = event instanceof DamagedEvent && ((DamagedEvent) event).isCombatDamage();
                    synchronized (tally) {
                        if (me.equals(game.getControllerId(event.getSourceId()))) {
                            tally.lastHit.add(event.getTargetId());
                        } else {
                            tally.lastHit.remove(event.getTargetId());
                        }
                    }
                    if (me.equals(event.getTargetId())) {
                        tally.add("damage_taken", event.getAmount());
                        Player player = game.getPlayer(me);
                        if (player != null) {
                            tally.lifeNow(player.getLife());
                        }
                    } else if (me.equals(game.getControllerId(event.getSourceId()))) {
                        tally.add("damage_to_opponents", event.getAmount());
                        tally.add(combat ? "combat_damage_to_opponents" : "noncombat_damage_to_opponents", event.getAmount());
                        tally.max("biggest_hit", event.getAmount());
                        tally.moment("big_hit", event.getAmount(), game.getTurnNum());
                        Permanent source = game.getPermanentOrLKIBattlefield(event.getSourceId());
                        if (combat && source != null && source.isCreature(game)) {
                            tally.max("max_attacker_power", source.getPower().getValue());
                            Player controller = game.getPlayer(me);
                            if (controller != null && game.getCommandersIds(controller, CommanderCardType.COMMANDER_OR_OATHBREAKER, false).contains(source.getId())) {
                                tally.add("commander_damage", event.getAmount());
                                tally.moment("commander_hit", event.getAmount(), game.getTurnNum());
                            }
                        }
                    }
                    break;
                }
                case LOST_LIFE:
                    if (me.equals(event.getTargetId())) {
                        Player player = game.getPlayer(me);
                        if (player != null) {
                            tally.lifeNow(player.getLife());
                        }
                    }
                    break;
                case GAINED_LIFE:
                    if (me.equals(event.getTargetId())) {
                        tally.add("life_gained", event.getAmount());
                        tally.moment("life_swing", event.getAmount(), game.getTurnNum());
                    }
                    break;
                case CREATED_TOKEN:
                    if (me.equals(event.getPlayerId())) {
                        tally.add("tokens_created", 1);
                    }
                    break;
                case LAND_PLAYED:
                    if (me.equals(event.getPlayerId())) {
                        tally.add("lands_played", 1);
                    }
                    break;
                case DREW_CARD:
                    if (me.equals(event.getPlayerId())) {
                        tally.add("cards_drawn", 1);
                    }
                    break;
                case ATTACKER_DECLARED:
                    if (me.equals(game.getControllerId(event.getSourceId()))) {
                        tally.add("attacks", 1);
                    }
                    break;
                case BLOCKER_DECLARED:
                    if (me.equals(game.getControllerId(event.getSourceId()))) {
                        tally.add("blocks", 1);
                    }
                    break;
                case COUNTERS_ADDED:
                    if (me.equals(game.getControllerId(event.getTargetId())) && game.getPermanent(event.getTargetId()) != null) {
                        tally.add("counters_added", event.getAmount());
                    }
                    break;
                case ZONE_CHANGE:
                    zoneChange(tally, (ZoneChangeEvent) event, game);
                    break;
                default:
                    break;
            }
        }

        private static void spell(CareerTally tally, Spell spell, Game game) {
            tally.add(SPELLS, 1);
            synchronized (tally) {
                tally.spellsThisTurn++;
                tally.counters.merge("most_spells_in_a_turn", tally.spellsThisTurn, Math::max);
            }
            tally.moment("storm", tally.spellsThisTurn, game.getTurnNum());
            if (spell == null) {
                return;
            }
            ObjectColor color = spell.getColor(game);
            if (color.isWhite()) {
                tally.add("spells_W", 1);
            }
            if (color.isBlue()) {
                tally.add("spells_U", 1);
            }
            if (color.isBlack()) {
                tally.add("spells_B", 1);
            }
            if (color.isRed()) {
                tally.add("spells_R", 1);
            }
            if (color.isGreen()) {
                tally.add("spells_G", 1);
            }
            if (color.getColorCount() == 0) {
                tally.add("spells_colorless", 1);
            } else if (color.getColorCount() > 1) {
                tally.add("spells_multicolor", 1);
            }
            if (spell.isCreature(game)) {
                tally.add("creature_spells", 1);
            }
            if (spell.isInstant(game) || spell.isSorcery(game)) {
                tally.add("instant_sorcery_spells", 1);
            }
            if (spell.isEnchantment(game)) {
                tally.add("enchantment_spells", 1);
            }
            if (spell.isArtifact(game)) {
                tally.add("artifact_spells", 1);
            }
            if (spell.isPlaneswalker(game)) {
                tally.add("planeswalker_spells", 1);
            }
            int manaValue = spell.getManaValue();
            if (manaValue >= 5) {
                tally.add("spells_mv5", 1);
            }
            if (manaValue <= 2) {
                tally.add("spells_cheap", 1);
            }
        }

        private static void zoneChange(CareerTally tally, ZoneChangeEvent event, Game game) {
            Permanent permanent = event.getTarget();
            if (event.getToZone() == Zone.BATTLEFIELD && permanent != null && permanent.isCreature(game)
                    && tally.playerId.equals(permanent.getControllerId())) {
                tally.add("creatures_entered", 1);
            }
            if (event.getFromZone() == Zone.BATTLEFIELD && event.getToZone() == Zone.GRAVEYARD && permanent != null
                    && permanent.isCreature(game) && !tally.playerId.equals(permanent.getControllerId())) {
                tally.add("opponent_creatures_died", 1);
            }
            // an opponent's card exiled by something this player controls
            if (event.getToZone() == Zone.EXILED && event.getFromZone() != Zone.STACK && event.getSourceId() != null
                    && tally.playerId.equals(game.getControllerId(event.getSourceId()))) {
                UUID owner = game.getOwnerId(event.getTargetId());
                if (owner != null && !tally.playerId.equals(owner)) {
                    tally.add("cards_exiled", 1);
                }
            }
        }
    }
}
