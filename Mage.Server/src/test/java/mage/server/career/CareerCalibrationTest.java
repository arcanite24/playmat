package mage.server.career;

import mage.cards.decks.Deck;
import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardScanner;
import mage.collectors.DataCollectorServices;
import mage.constants.MultiplayerAttackOption;
import mage.constants.PhaseStep;
import mage.constants.RangeOfInfluence;
import mage.game.Game;
import mage.game.GameOptions;
import mage.game.TwoPlayerDuel;
import mage.game.TwoPlayerMatch;
import mage.game.match.Match;
import mage.game.match.MatchOptions;
import mage.game.mulligan.MulliganType;
import mage.player.ai.ComputerPlayerControllableProxy;
import mage.players.Player;
import mage.util.ThreadUtils;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicReference;
import java.util.stream.Collectors;

/**
 * Career difficulty calibration (backlog B093): plays whole AI-vs-AI games of the starter decks against every Career
 * opponent and prints the starter side's win rate per opponent and per tier. The starter decks are played by a fixed
 * reference AI standing in for a new player; each opponent plays its own deck at its configured skill, exactly as a
 * Career match seats it (same AI class, same skill resolution as CareerApi).
 * <p>
 * Skipped by default; it takes a long time. Run with (see career/CALIBRATION.md):
 * <pre>
 * mvn -o -pl Mage.Server.Plugins/Mage.Player.AI.MAD,Mage.Server test -Dtest=CareerCalibrationTest \
 *     -Dsurefire.failIfNoSpecifiedTests=false -Dxmage.careerCalibration=true \
 *     -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$PWD/.travis/log4j.properties
 * </pre>
 * Options (system properties):
 * <ul>
 * <li>xmage.careerCalibration.games: games per opponent, the starters take turns (default 12, one per starter)</li>
 * <li>xmage.careerCalibration.referenceSkill: skill of the AI playing the starters (default 4)</li>
 * <li>xmage.careerCalibration.maxTurns: a game still going at this turn is a draw (default 30, both players' turns)</li>
 * <li>xmage.careerCalibration.opponents: comma separated opponent ids to play (default all)</li>
 * <li>xmage.careerCalibration.tierSkills: try other tier skills without editing opponents.json, e.g. 1,4,7</li>
 * <li>xmage.careerCalibration.shard: i/n plays only every n-th game starting at i (0-based), to split one run over
 * several JVMs; each game prints a RESULT line, and {@link #main} with the shard logs as arguments merges them</li>
 * <li>xmage.careerCalibration.schools: calibrate the format schools instead (backlog B159): "all" or comma separated
 * school ids. Every one-on-one duel plays the node's loaner at the reference skill against the node's opponent at its
 * skill; the summary's "tier" is the act. Commander and Brawl games play with a command zone, and pods seat every
 * opponent of the node (the starter side scores a win only when it is the last one standing).</li>
 * <li>xmage.careerCalibration.opponentAi: "mad" (default) or "mcts", the Monte Carlo AI, to measure it in the opponent's
 * seat (S044); an opponent with its own aiType plays with that unless this is set</li>
 * <li>xmage.careerCalibration.personalities: false plays every opponent without its personality, to measure what the
 * personalities change (S042)</li>
 * </ul>
 * Opponents play their lists in turn (game i plays list i), start with their tier's and their own life twists, and
 * carry their personality, as a Career duel does.
 */
public class CareerCalibrationTest {

    private static final String PREFIX = "xmage.careerCalibration";
    private static final long GAME_TIMEOUT_MS = 20 * 60 * 1000L;
    // card lookups are cached, so after the first games the card database is no longer needed (several JVMs
    // share one database, served by whichever opened it first, and that one can finish early)
    private static final Map<String, mage.cards.repository.CardInfo> CARD_INFO_CACHE = new java.util.concurrent.ConcurrentHashMap<>();

    @Test
    public void calibrateTiers() throws Exception {
        Assumptions.assumeTrue(Boolean.getBoolean(PREFIX), "career calibration runs only with -D" + PREFIX + "=true");
        run();
    }

    /**
     * Runs the calibration outside JUnit (one JVM per shard), or with arguments, merges the RESULT lines of
     * earlier runs' logs into one summary: {@code CareerCalibrationTest shard0.log shard1.log ...}
     */
    public static void main(String[] args) throws Exception {
        if (args.length > 0) {
            List<Result> results = new ArrayList<>();
            for (String file : args) {
                for (String line : java.nio.file.Files.readAllLines(java.nio.file.Paths.get(file))) {
                    if (line.startsWith("RESULT,")) {
                        results.add(parse(line));
                    }
                }
            }
            System.out.println(summary(results));
            return;
        }
        run();
    }

    private static Result parse(String line) {
        // RESULT,tier,opponent,skill,starter,outcome,turns,seconds,first|second
        String[] parts = line.split(",");
        Result result = new Result();
        result.tier = Integer.parseInt(parts[1]);
        result.opponent = parts[2];
        result.skill = Integer.parseInt(parts[3]);
        result.starter = parts[4];
        result.outcome = parts[5];
        result.turns = Integer.parseInt(parts[6]);
        return result;
    }

    private static void run() throws Exception {
        if (!System.getProperty(PREFIX + ".schools", "").isEmpty()) {
            runSchools();
            return;
        }
        int gamesPerOpponent = Integer.getInteger(PREFIX + ".games", 12);
        int referenceSkill = Integer.getInteger(PREFIX + ".referenceSkill", 4);
        int maxTurns = Integer.getInteger(PREFIX + ".maxTurns", 30);
        String onlyOpponents = System.getProperty(PREFIX + ".opponents", "");
        String tierSkills = System.getProperty(PREFIX + ".tierSkills", "");
        String shard = System.getProperty(PREFIX + ".shard", "0/1");
        int shardIndex = Integer.parseInt(shard.split("/")[0]);
        int shardCount = Integer.parseInt(shard.split("/")[1]);

        CardScanner.scan();
        DataCollectorServices.init(false, false);

        CareerContent content = CareerContent.get();
        List<CareerContent.CareerStarter> starters = content.starters();
        Map<String, DeckCardLists> starterDecks = new LinkedHashMap<>();
        for (CareerContent.CareerStarter starter : starters) {
            starterDecks.put(starter.id, content.starterDeck(starter.id));
            Deck.load(starterDecks.get(starter.id), false, false, CARD_INFO_CACHE);
        }
        Map<Integer, Integer> skillOverride = new HashMap<>();
        if (!tierSkills.isEmpty()) {
            String[] parts = tierSkills.split(",");
            for (int i = 0; i < parts.length; i++) {
                skillOverride.put(i + 1, Integer.parseInt(parts[i].trim()));
            }
        }
        List<String> filter = onlyOpponents.isEmpty() ? null : Arrays.asList(onlyOpponents.split(","));

        System.out.println(String.format("CALIBRATION reference skill %d, %d games per opponent, draw at turn %d, shard %s",
                referenceSkill, gamesPerOpponent, maxTurns, shard));
        List<Result> results = new ArrayList<>();
        int gameIndex = 0;
        for (CareerContent.Opponent opponent : content.opponents()) {
            if (filter != null && !filter.contains(opponent.id)) {
                continue;
            }
            int skill = skillOverride.getOrDefault(opponent.tier,
                    opponent.skill != null ? opponent.skill : content.tier(opponent.tier).skill);
            String opponentAi = System.getProperty(PREFIX + ".opponentAi",
                    "archmage".equals(opponent.aiType) || opponent.aiType == null ? "mad" : opponent.aiType);
            Map<String, Integer> personality = Boolean.parseBoolean(System.getProperty(PREFIX + ".personalities", "true"))
                    && opponent.personality != null ? personality(opponent.personality) : null;
            CareerSetup.CareerTwists twists = CareerSetup.combine(content.tier(opponent.tier).twists, opponent.twists);
            int opponentLife = twists == null ? 20 : (twists.startingLife != null ? twists.startingLife : 20) + twists.lifeBonus;
            for (int i = 0; i < gamesPerOpponent; i++, gameIndex++) {
                if (gameIndex % shardCount != shardIndex) {
                    continue;
                }
                // the lists take turns, as they do across a player's meetings
                DeckCardLists opponentDeck = content.opponentDeck(opponent, i);
                Deck.load(opponentDeck, false, false, CARD_INFO_CACHE);
                CareerContent.CareerStarter starter = starters.get(i % starters.size());
                // the starter side goes first in every other game
                boolean starterFirst = (i / starters.size() + i) % 2 == 0;
                long start = System.currentTimeMillis();
                Result result;
                try {
                    result = playGame(null, starterDecks.get(starter.id), referenceSkill,
                            java.util.Collections.singletonList(new Seat(opponentDeck, skill, opponentAi, personality)),
                            starterFirst, maxTurns, opponentLife);
                } catch (Exception e) {
                    e.printStackTrace();
                    result = new Result();
                    result.outcome = "error";
                }
                result.opponent = opponent.id;
                result.tier = opponent.tier;
                result.skill = skill;
                result.starter = starter.id;
                results.add(result);
                System.out.println(String.format("RESULT,%d,%s,%d,%s,%s,%d,%d,%s",
                        result.tier, result.opponent, result.skill, result.starter, result.outcome, result.turns,
                        (System.currentTimeMillis() - start) / 1000, starterFirst ? "first" : "second"));
            }
        }
        System.out.println(summary(results));
    }

    /** the schools' one-on-one duels: each node's loaner (the "starter" side) against its opponent */
    private static void runSchools() throws Exception {
        int gamesPerNode = Integer.getInteger(PREFIX + ".games", 4);
        int referenceSkill = Integer.getInteger(PREFIX + ".referenceSkill", 4);
        int maxTurns = Integer.getInteger(PREFIX + ".maxTurns", 30);
        String only = System.getProperty(PREFIX + ".schools", "all");
        String onlyNodes = System.getProperty(PREFIX + ".nodes", "");
        String shard = System.getProperty(PREFIX + ".shard", "0/1");
        int shardIndex = Integer.parseInt(shard.split("/")[0]);
        int shardCount = Integer.parseInt(shard.split("/")[1]);

        CardScanner.scan();
        DataCollectorServices.init(false, false);
        CareerCampaigns campaigns = CareerCampaigns.get();
        System.out.println(String.format("CALIBRATION schools %s, reference skill %d, %d games per duel, draw at turn %d, shard %s",
                only, referenceSkill, gamesPerNode, maxTurns, shard));
        List<Result> results = new ArrayList<>();
        int gameIndex = 0;
        for (CareerCampaigns.Campaign school : campaigns.campaigns().values()) {
            if (!school.school || !("all".equals(only) || Arrays.asList(only.split(",")).contains(school.id))) {
                continue;
            }
            for (int act = 0; act < school.chapters.size(); act++) {
                CareerCampaigns.Chapter chapter = school.chapters.get(act);
                for (CareerCampaigns.Node node : chapter.nodes) {
                    String gameType = node.gameType != null ? node.gameType : school.gameType;
                    boolean pod = node.opponents != null && !node.opponents.isEmpty();
                    if (!"duel".equals(node.type) || (!pod && node.opponent == null)
                            || (!onlyNodes.isEmpty() && !Arrays.asList(onlyNodes.split(",")).contains(node.id))) {
                        continue;
                    }
                    DeckCardLists you = campaigns.deck(node.deck != null ? node.deck : chapter.deck);
                    // a pod seats each of its opponents; a duel its one
                    List<CareerCampaigns.NodeOpponent> seated = pod ? node.opponents : java.util.Collections.singletonList(node.opponent);
                    List<Seat> them = new ArrayList<>();
                    for (CareerCampaigns.NodeOpponent opponent : seated) {
                        them.add(new Seat(campaigns.deck(opponent.deck), opponent.skill, "mad", null));
                    }
                    for (int i = 0; i < gamesPerNode; i++, gameIndex++) {
                        if (gameIndex % shardCount != shardIndex) {
                            continue;
                        }
                        boolean youFirst = i % 2 == 0;
                        long start = System.currentTimeMillis();
                        Result result;
                        try {
                            result = playGame(gameType, you, referenceSkill, them, youFirst, maxTurns, 0);
                        } catch (Exception e) {
                            e.printStackTrace();
                            result = new Result();
                            result.outcome = "error";
                        }
                        result.tier = act + 1;
                        result.opponent = node.id;
                        result.skill = seated.get(0).skill;
                        result.starter = school.id;
                        results.add(result);
                        System.out.println(String.format("RESULT,%d,%s,%d,%s,%s,%d,%d,%s",
                                result.tier, result.opponent, result.skill, result.starter, result.outcome, result.turns,
                                (System.currentTimeMillis() - start) / 1000, youFirst ? "first" : "second"));
                    }
                }
            }
        }
        System.out.println(summary(results));
    }

    static class Result {
        int tier;
        String opponent;
        int skill;
        String starter;
        String outcome; // win, loss, draw (for the starter side) or error
        int turns;
    }

    private static Map<String, Integer> personality(CareerSetup.Personality personality) {
        Map<String, Integer> leanings = new HashMap<>();
        leanings.put("aggression", personality.aggression);
        leanings.put("risk", personality.risk);
        leanings.put("patience", personality.patience);
        return leanings;
    }

    private static Player opponentPlayer(String ai, int skill, String name) throws Exception {
        if ("mcts".equalsIgnoreCase(ai) || "COMPUTER_MONTE_CARLO".equals(ai)) {
            // the Monte Carlo AI is a runtime plugin, so it's found by name
            return (Player) Class.forName("mage.player.ai.ComputerPlayerMCTS")
                    .getConstructor(String.class, RangeOfInfluence.class, int.class)
                    .newInstance(name, RangeOfInfluence.ALL, skill);
        }
        return new ComputerPlayerControllableProxy(name, RangeOfInfluence.ALL, skill);
    }

    /** an opponent's seat: deck, skill, AI and leanings */
    static final class Seat {
        final DeckCardLists deck;
        final int skill;
        final String ai;
        final Map<String, Integer> personality;

        Seat(DeckCardLists deck, int skill, String ai, Map<String, Integer> personality) {
            this.deck = deck;
            this.skill = skill;
            this.ai = ai;
            this.personality = personality;
        }
    }

    /**
     * The game a node's type plays: Commander and Brawl games have a command zone (the deck's sideboard holds the
     * commander) and their own starting life, and pods are free for all. Null or anything else is a two-player duel.
     */
    private static Game newGame(String gameType, AtomicBoolean crashed) throws Exception {
        String type = gameType == null ? "" : gameType;
        boolean pod = type.contains("Free For All");
        String className = type.startsWith("Commander") ? (pod ? "mage.game.CommanderFreeForAll" : "mage.game.CommanderDuel")
                : type.startsWith("Brawl") ? (pod ? "mage.game.BrawlFreeForAll" : "mage.game.BrawlDuel")
                : pod ? "mage.game.FreeForAll" : null;
        if (className == null) {
            return new TwoPlayerDuel(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL,
                    MulliganType.GAME_DEFAULT.getMulligan(0), 60, 20, 7) {
                @Override
                public void endWithTechnicalWinner(String reason) {
                    // the engine gave up on the game: its winner says nothing about the decks
                    crashed.set(true);
                    super.endWithTechnicalWinner(reason);
                }
            };
        }
        int life = type.startsWith("Commander") ? 40 : type.startsWith("Brawl") ? (pod ? 30 : 25) : 20;
        // the plugins are runtime dependencies, so they're found by name
        return (Game) Class.forName(className)
                .getConstructor(MultiplayerAttackOption.class, RangeOfInfluence.class, mage.game.mulligan.Mulligan.class, int.class, int.class)
                .newInstance(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL, MulliganType.GAME_DEFAULT.getMulligan(0), life, 7);
    }

    /**
     * @param opponentLife the opponents' starting life when a twist sets it, or 0 for the game's own
     */
    private static Result playGame(String gameType, DeckCardLists starterList, int referenceSkill, List<Seat> seats,
                                   boolean starterFirst, int maxTurns, int opponentLife) throws Exception {
        AtomicBoolean crashed = new AtomicBoolean();
        Game game = newGame(gameType, crashed);
        // the AI's simulations copy the match player, so seat everyone in a (fake) match as a table would
        Match match = new TwoPlayerMatch(new MatchOptions("calibration", "Two Player Duel", false));
        Player starterPlayer = new ComputerPlayerControllableProxy("Starter", RangeOfInfluence.ALL, referenceSkill);
        seat(game, match, starterPlayer, starterList);
        List<Player> opponents = new ArrayList<>();
        for (int i = 0; i < seats.size(); i++) {
            Seat seat = seats.get(i);
            Player opponent = opponentPlayer(seat.ai, seat.skill, seats.size() == 1 ? "Opponent" : "Opponent " + (i + 1));
            seat(game, match, opponent, seat.deck);
            if (seat.personality != null) {
                game.getState().setValue(CareerSetup.PERSONALITY_KEY + opponent.getId(), seat.personality);
            }
            opponents.add(opponent);
        }
        Player opponentPlayer = opponents.get(0);
        GameOptions options = new GameOptions();
        options.testMode = false; // real mulligans and opening hands
        options.stopOnTurn = maxTurns;
        options.stopAtStep = PhaseStep.UNTAP;
        if (opponentLife > 0 && opponentLife != game.getStartingLife()) {
            // the tier's and the opponent's life twists (a raid boss's 100)
            java.util.UUID opponentId = opponentPlayer.getId();
            options.setup = (mage.game.GameSetup) (setupGame, startingPlayerId) ->
                    setupGame.getPlayer(opponentId).setLife(opponentLife, setupGame, null);
        }
        game.setGameOptions(options);

        Result result = new Result();
        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread thread = new Thread(() -> {
            try {
                game.start(starterFirst ? starterPlayer.getId() : opponentPlayer.getId());
            } catch (Throwable e) {
                failure.set(e);
            }
        }, ThreadUtils.THREAD_PREFIX_GAME + " calibration");
        thread.start();
        thread.join(GAME_TIMEOUT_MS);
        if (thread.isAlive()) {
            // a stuck game: stop it as well as we can and leave it out of the numbers
            thread.interrupt();
            game.setGameStopped(true);
            thread.join(10_000);
            result.outcome = "error";
            result.turns = game.getTurnNum();
            return result;
        }
        result.turns = game.getTurnNum();
        if (failure.get() != null || crashed.get()) {
            if (failure.get() != null) {
                failure.get().printStackTrace();
            }
            result.outcome = "error";
        } else if (game.getPlayer(starterPlayer.getId()).hasWon()) {
            result.outcome = "win";
        } else if (opponents.stream().anyMatch(opponent -> game.getPlayer(opponent.getId()).hasWon())) {
            result.outcome = "loss";
        } else {
            result.outcome = "draw";
        }
        return result;
    }

    private static void seat(Game game, Match match, Player player, DeckCardLists list) throws Exception {
        Deck deck = Deck.load(list, false, false, CARD_INFO_CACHE);
        game.loadCards(deck.getCards(), player.getId());
        game.loadCards(deck.getSideboard(), player.getId());
        game.addPlayer(player, deck);
        match.addPlayer(player, deck);
    }

    /** starter-side score: a win counts 1, a draw (turn limit) counts half */
    static String summary(List<Result> results) {
        StringBuilder sb = new StringBuilder("\nCALIBRATION SUMMARY (starter side)\n");
        sb.append(String.format("%-10s %-5s %-5s %5s %5s %5s %5s %6s%n", "opponent", "tier", "skill", "games", "win", "loss", "draw", "score"));
        Map<String, List<Result>> byOpponent = results.stream()
                .filter(r -> !"error".equals(r.outcome))
                .collect(Collectors.groupingBy(r -> r.opponent, LinkedHashMap::new, Collectors.toList()));
        for (Map.Entry<String, List<Result>> entry : byOpponent.entrySet()) {
            Result first = entry.getValue().get(0);
            sb.append(line(entry.getKey(), String.valueOf(first.tier), String.valueOf(first.skill), entry.getValue()));
        }
        Map<Integer, List<Result>> byTier = results.stream()
                .filter(r -> !"error".equals(r.outcome))
                .collect(Collectors.groupingBy(r -> r.tier, java.util.TreeMap::new, Collectors.toList()));
        for (Map.Entry<Integer, List<Result>> entry : byTier.entrySet()) {
            sb.append(line("TIER " + entry.getKey(), String.valueOf(entry.getKey()), "", entry.getValue()));
        }
        long errors = results.stream().filter(r -> "error".equals(r.outcome)).count();
        sb.append("errors (not counted): ").append(errors).append('\n');
        return sb.toString();
    }

    private static String line(String name, String tier, String skill, List<Result> games) {
        long win = games.stream().filter(r -> "win".equals(r.outcome)).count();
        long loss = games.stream().filter(r -> "loss".equals(r.outcome)).count();
        long draw = games.stream().filter(r -> "draw".equals(r.outcome)).count();
        double score = games.isEmpty() ? 0 : (win + draw / 2.0) / games.size();
        return String.format("%-10s %-5s %-5s %5d %5d %5d %5d %5.0f%%%n", name, tier, skill, games.size(), win, loss, draw, score * 100);
    }
}
