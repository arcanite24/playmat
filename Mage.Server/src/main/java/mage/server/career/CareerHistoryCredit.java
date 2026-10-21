package mage.server.career;

import mage.game.result.ResultProtos;
import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.time.Instant;
import java.time.ZoneOffset;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

/**
 * Credit for games played before progression counted them (B308). Once per server, the table records (the same ones
 * the ratings come from) are read for every human player: games and wins, by kind (against the AI, a person, a pod,
 * an event) and format, best-of-three matches won, events played and days played. Each lifetime counter is raised to
 * what history proves, never lowered and never added twice, so games progression already counted aren't doubled.
 * Achievements pick the counts up the next time the player finishes a game.
 */
public final class CareerHistoryCredit {

    private static final Logger logger = Logger.getLogger(CareerHistoryCredit.class);
    static final String DONE = "history_credit";

    private CareerHistoryCredit() {
    }

    /** runs the credit in the background, once per server */
    public static void startOnce() {
        Thread thread = new Thread(() -> {
            try {
                CareerStore store = CareerStore.get();
                if (store.metaValue(DONE) != null) {
                    return;
                }
                List<mage.server.record.TableRecord> records = mage.server.record.TableRecordRepository.instance.getAfter(0);
                java.util.List<ResultProtos.TableProto> tables = new java.util.ArrayList<>();
                for (mage.server.record.TableRecord record : records) {
                    tables.add(record.getProto());
                }
                int players = credit(store, tables);
                logger.info("Career: credited past games to " + players + " players from " + tables.size() + " table records");
            } catch (SQLException | RuntimeException e) {
                logger.warn("Career: past games couldn't be credited", e);
            }
        }, "Career history credit");
        thread.setDaemon(true);
        thread.start();
    }

    /**
     * Credits these table records and marks the credit done.
     *
     * @return how many players got credit
     */
    static int credit(CareerStore store, List<ResultProtos.TableProto> tables) throws SQLException {
        Map<String, Map<String, Integer>> counts = new HashMap<>();
        Map<String, Set<Integer>> days = new HashMap<>();
        Map<String, String> names = new HashMap<>();
        for (ResultProtos.TableProto table : tables) {
            int day = day(table.getEndTimeMs());
            if (table.hasMatch()) {
                // a match inside an event is a sub table controlled by "System"
                boolean event = "System".equals(table.getControllerName());
                match(table.getMatch(), event, day, counts, days, names);
            } else if (table.hasTourney()) {
                for (ResultProtos.TourneyPlayerProto player : table.getTourney().getPlayersList()) {
                    if (player.getPlayerType().toLowerCase(Locale.ROOT).contains("human")) {
                        String user = remember(player.getName(), names);
                        add(counts, user, "events_played", 1);
                    }
                }
            }
        }
        for (Map.Entry<String, Set<Integer>> entry : days.entrySet()) {
            add(counts, entry.getKey(), "days_played", entry.getValue().size());
            add(counts, entry.getKey(), "last_day", entry.getValue().stream().max(Integer::compare).orElse(0));
        }
        int players = 0;
        for (Map.Entry<String, Map<String, Integer>> entry : counts.entrySet()) {
            String user = names.get(entry.getKey());
            store.transaction(() -> {
                for (Map.Entry<String, Integer> counter : entry.getValue().entrySet()) {
                    store.maxStat(user, counter.getKey(), counter.getValue());
                }
                return null;
            });
            players++;
        }
        store.setMeta(DONE, String.valueOf(System.currentTimeMillis()));
        return players;
    }

    private static void match(ResultProtos.MatchProto match, boolean event, int day, Map<String, Map<String, Integer>> counts,
                              Map<String, Set<Integer>> days, Map<String, String> names) {
        List<ResultProtos.MatchPlayerProto> players = match.getPlayersList();
        long humans = players.stream().filter(ResultProtos.MatchPlayerProto::getHuman).count();
        int games = match.getGames();
        if (games <= 0) {
            games = match.getDraws();
            for (ResultProtos.MatchPlayerProto player : players) {
                games += player.getWins();
            }
        }
        if (games <= 0) {
            return;
        }
        String kind = event ? CareerRewards.EVENT
                : humans < players.size() ? CareerRewards.AI
                : players.size() > 2 ? CareerRewards.POD : CareerRewards.PERSON;
        String format = CareerService.formatOf(match.getDeckType(), match.getGameType(), match.getMatchOptions().getLimited());
        int winsNeeded = match.getMatchOptions().getWinsNeeded();
        for (ResultProtos.MatchPlayerProto player : players) {
            if (!player.getHuman() || player.getBye()) {
                continue;
            }
            String user = remember(player.getName(), names);
            int wins = Math.min(games, player.getWins());
            add(counts, user, CareerTally.GAMES, games);
            add(counts, user, CareerTally.WINS, wins);
            add(counts, user, "games_kind_" + kind, games);
            add(counts, user, "wins_kind_" + kind, wins);
            if (CareerService.FORMATS.contains(format)) {
                add(counts, user, "games_format_" + format, games);
                add(counts, user, "wins_format_" + format, wins);
            }
            if (winsNeeded >= 2 && player.getWins() >= winsNeeded) {
                add(counts, user, "bo3_match_wins", 1);
            }
            days.computeIfAbsent(user, key -> new HashSet<>()).add(day);
        }
    }

    private static String remember(String name, Map<String, String> names) {
        String key = CareerStore.key(name);
        names.putIfAbsent(key, name);
        return key;
    }

    private static void add(Map<String, Map<String, Integer>> counts, String user, String counter, int amount) {
        Map<String, Integer> mine = counts.computeIfAbsent(user, key -> new HashMap<>());
        if ("last_day".equals(counter)) {
            mine.merge(counter, amount, Math::max);
        } else {
            mine.merge(counter, amount, Integer::sum);
        }
    }

    static int day(long millis) {
        java.time.LocalDate date = Instant.ofEpochMilli(millis).atZone(ZoneOffset.UTC).toLocalDate();
        return date.getYear() * 10000 + date.getMonthValue() * 100 + date.getDayOfMonth();
    }
}
