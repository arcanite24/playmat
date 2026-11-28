package mage.server.career;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;

/**
 * A finished game as a payout needs it (docs/rust/Payouts.md, "The contract"): what the game's live state said, read
 * once at its end, so the payout itself needs only career.db, solo.db and the content. Java keeps it in
 * {@code payout_events} and pays from it; the Rust platform service reads the same rows and pays (or, in shadow,
 * replays) the same way. Gson writes it; Rust reads it with serde.
 */
public class CareerPayoutEvent {

    public static final String MATCH_ENDED = "matchEnded";
    public static final String GAME_ENDED = "gameEnded";

    /** {@link #MATCH_ENDED} (a Career table) or {@link #GAME_ENDED} (any other game) */
    public String event;
    /** the table (a Career match) or the game; the row's key */
    public String key;
    /** when the game ended (epoch millis); the payout's time */
    public long at;
    /** the UTC day it ended on, which the daily bonuses, quests and pair caps count */
    public String day;
    /** the admin's XP and coin settings (percent) when it ended */
    public int xpPercent = 100;
    public int coinPercent = 100;
    /** a Career match: whose and what it was */
    public Career career;
    /** another game: its table and seats */
    public Finished finished;
    /** a Career match: won, and the turn the last game ended on */
    public boolean won;
    public int turns;
    /** a Career match: the tally that counts (empty when tainted, assisted, a puzzle or a trial) */
    public Map<String, Integer> counters = new LinkedHashMap<>();
    public List<Moment> moments = new ArrayList<>();

    public static class Career {
        public String user;
        public String kind;
        public String opponent;
        public int tier;
        public String ref;
        public String assisted;
        /** when the table was made */
        public long created;
    }

    public static class Finished {
        /** {@link CareerRewards#AI}, PERSON, POD or EVENT */
        public String kind;
        public int players;
        public int turns;
        public int aiSkill;
        public boolean bo3;
        public String format;
        public List<Seat> seats = new ArrayList<>();
    }

    public static class Seat {
        public String user;
        public boolean won;
        public boolean matchWon;
        public List<String> colors = new ArrayList<>();
        /** the seat has a clean tally (else it gets nothing) */
        public boolean counted;
        public Map<String, Integer> counters = new LinkedHashMap<>();
        public List<Moment> moments = new ArrayList<>();
    }

    public static class Moment {
        public String id;
        public int value;
        public int turn;

        public Moment() {
        }

        Moment(CareerTally.Moment moment) {
            this.id = moment.id;
            this.value = moment.value;
            this.turn = moment.turn;
        }
    }

    /** a Career match that ended: the tally must already be finished (or empty) */
    static CareerPayoutEvent match(UUID tableId, CareerService.CareerTable table, boolean won, int turns, CareerTally counted, long now) {
        CareerPayoutEvent event = base(MATCH_ENDED, tableId.toString(), now);
        event.career = new Career();
        event.career.user = table.user;
        event.career.kind = table.kind;
        event.career.opponent = table.opponentId;
        event.career.tier = table.tier;
        event.career.ref = table.ref;
        event.career.assisted = table.assisted;
        event.career.created = table.created;
        event.won = won;
        event.turns = turns;
        event.counters.putAll(counted.counters());
        counted.moments().forEach(moment -> event.moments.add(new Moment(moment)));
        return event;
    }

    /** a game outside Career tables: each counted seat's tally gets its context and its end here */
    static CareerPayoutEvent game(CareerService.FinishedGame game, long now) {
        CareerPayoutEvent event = base(GAME_ENDED, game.gameId.toString(), now);
        event.finished = new Finished();
        event.finished.kind = game.kind;
        event.finished.players = game.players;
        event.finished.turns = game.turns;
        event.finished.aiSkill = game.aiSkill;
        event.finished.bo3 = game.bo3;
        event.finished.format = game.format;
        for (CareerService.Seat seat : game.seats) {
            Seat out = new Seat();
            out.user = seat.user;
            out.won = seat.won;
            out.matchWon = seat.matchWon;
            out.colors.addAll(seat.colors);
            out.counted = seat.tally != null && seat.tally.taint() == null;
            if (out.counted) {
                seat.tally.context(game.kind, game.format, seat.colors, seat.won, game.bo3 && seat.matchWon);
                seat.tally.finish(game.game, seat.won, game.turns);
                out.counters.putAll(seat.tally.counters());
                seat.tally.moments().forEach(moment -> out.moments.add(new Moment(moment)));
            }
            event.finished.seats.add(out);
        }
        return event;
    }

    private static CareerPayoutEvent base(String kind, String key, long now) {
        CareerPayoutEvent event = new CareerPayoutEvent();
        event.event = kind;
        event.key = key;
        event.at = now;
        event.day = CareerProgress.get().day();
        event.xpPercent = CareerRewards.xpPercent();
        event.coinPercent = CareerRewards.coinPercent();
        return event;
    }

    /** the tally as it was paid: counters and moments only (the game is over) */
    static CareerTally tally(Map<String, Integer> counters, List<Moment> moments) {
        CareerTally tally = CareerTally.forTest(null);
        if (counters != null) {
            counters.forEach(tally::set);
        }
        if (moments != null) {
            for (Moment moment : moments) {
                tally.restore(moment.id, moment.value, moment.turn);
            }
        }
        return tally;
    }

    /** the seats whose users the payout touches, in order */
    List<String> users() {
        List<String> users = new ArrayList<>();
        if (career != null) {
            users.add(career.user);
        }
        if (finished != null) {
            for (Seat seat : finished.seats) {
                users.add(seat.user);
            }
        }
        return users;
    }
}
