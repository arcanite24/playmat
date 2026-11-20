package mage.server.websocket.api;

import mage.cards.decks.DeckCardLists;
import mage.players.PlayerType;
import mage.server.career.CareerContent;
import mage.server.career.CareerLeague;
import mage.server.career.CareerProgress;
import mage.server.ladder.BotRoster;
import mage.server.ladder.DeckLegality;
import mage.server.ladder.LadderRank;
import mage.server.ladder.LadderStore;
import mage.server.ladder.RankedService;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;
import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.UUID;

import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.object;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Ranked (#431): the Open queue against the ladder's bots, one rank per format. Signed-in accounts on servers that
 * offer Career; what climbing pays goes into the Career profile.
 */
public final class LadderApi {

    private static final Logger logger = Logger.getLogger(LadderApi.class);
    /** the ranked games listed with the standing */
    static final int RECENT = 10;

    private LadderApi() {
    }

    /** A bot as the player meets it */
    public static class LadderOpponent {
        public String botId;
        public String name;
        public String avatar;
        public String sleeve;
        public String playmat;
        public String frame;
        public String title;
        public String deckName;
        /** WUBRG letters from the deck's basic lands */
        public String colors;
        /** how hard it searches: quick, light, half or full */
        public String search;
    }

    /** A ranked game, as listed */
    public static class LadderGame {
        public String id;
        /** playing, won, lost, drew or void */
        public String status;
        public LadderOpponent opponent;
        public String deck;
        public Integer stepBefore;
        public Integer stepAfter;
        public long started;
        public Long ended;
    }

    /** A player's standing in one format's Open queue */
    public static class LadderState {
        /** the formats with a ladder */
        public List<String> formats;
        public String format;
        public String queue;
        /** bronze … mythic */
        public String rank;
        /** 4 (lowest) to 1; 0 in Mythic */
        public int tier;
        public int pips;
        public int pipsPerTier;
        public int step;
        public int mythicStep;
        public String label;
        public int peakStep;
        public String peakLabel;
        public int streak;
        public int won;
        public int lost;
        public int drew;
        /** the hidden rating, shown in Mythic only */
        public Integer mythicRating;
        /** a ranked game is being played now */
        public boolean playing;
        public List<LadderGame> recent;
        /** the season now, counted from 0 (#434) */
        public int season;
        /** "Season 1" */
        public String seasonName;
        /** when the season ends, ms UTC */
        public long seasonEnds;
        /** whole days until it ends, at least 1 */
        public int seasonDaysLeft;
        /** the last season the player played here before this one, with where this one placed them; null if none */
        public LadderPastSeason lastSeason;
        /** every earlier season played here, newest first: the badge's season stamps */
        public List<LadderPastSeason> seasons;
    }

    /** A season the player played in a format and queue: how it ended, and (the last one) what its end paid */
    public static class LadderPastSeason {
        public int season;
        public String seasonName;
        public int peakStep;
        public String peakRank;
        public String peakLabel;
        public String endLabel;
        /** where the next season placed the player (the last season only) */
        public String placedLabel;
        /** League packs and mythic wildcards its end paid (the League only) */
        public int packs;
        public int mythicWildcards;
    }

    /** A ranked match was set up; the game starts like any other */
    public static class LadderMatch {
        public String tableId;
        public LadderOpponent opponent;
        public List<CareerContent.CareerCover> warm;
    }

    /** What a ranked game did, for the screen after it */
    public static class LadderResult {
        public String tableId;
        public String format;
        /** won, lost or drew */
        public String status;
        public LadderOpponent opponent;
        public int stepBefore;
        public int stepAfter;
        public String labelBefore;
        public String labelAfter;
        public int delta;
        /** a streak's bonus pip */
        public boolean bonus;
        public int streak;
        /** ranks reached for the first time */
        public List<String> reached;
        public List<CareerProgress.CareerCosmetic> cosmetics;
        public int tokens;
        /** the queue: open, bo3 or league */
        public String queue;
        /** League pack tokens of the format the game paid */
        public int packs;
        /** League wildcards the game paid, by rarity */
        public java.util.Map<String, Integer> wildcards;
        public Integer mythicRating;
    }

    public static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("ladderState")
                        .params(of("format", STRING), optional("queue", STRING))
                        .returns("LadderState")
                        .doc("Your ranked standing in a format's queue: rank, tier, pips, peak, record, the last games, "
                                + "and in Mythic your rating. Formats: standard, pioneer, modern, and limited (the rank Career's sealed and draft runs move, Open queue only, no rating). Queue: open (Bo1, the default), bo3 (best of "
                                + "three with sideboarding, its own rank) or league, the Career League.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String format = format(call.string(0));
                            String queue = queue(call.optString(1, RankedService.OPEN));
                            return ladder(() -> state(user, format, queue, System.currentTimeMillis()));
                        }),

                RpcMethod.named("ladderOpponent")
                        .params(of("format", STRING), optional("queue", STRING))
                        .returns("LadderOpponent")
                        .doc("Who your next ranked game in the format is against: a bot near your hidden rating. The pick holds "
                                + "until it's played, so it can't be skipped. Queue: open (the default), bo3 or league.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String format = queued(call.string(0));
                            String queue = queue(call.optString(1, RankedService.OPEN));
                            return ladder(() -> opponent(RankedService.get().pick(user, format, queue, System.currentTimeMillis())));
                        }),

                RpcMethod.named("ladderPlay")
                        .params(of("format", STRING), object("deck", "DeckCardLists"), optional("queue", STRING))
                        .returns("LadderMatch")
                        .doc("Start a ranked game in the format with a deck legal there, against the picked bot. Queue: open "
                                + "(Bo1, the default) or bo3 (best of three; the sideboard comes into play between games). No hints, "
                                + "take-backs, practice tools or saves; conceding or leaving is a loss.")
                        .handler(call -> play(ctx, call)),

                RpcMethod.named("leaguePlay")
                        .params(of("format", STRING), of("deckId", STRING))
                        .returns("LadderMatch")
                        .doc("Start a League game in the format with one of your League decks (by id), against the League's picked "
                                + "bot. The deck must be legal there and every card in it owned. Otherwise like ladderPlay.")
                        .handler(call -> leaguePlay(ctx, call)),

                RpcMethod.named("ladderResult")
                        .params(of("tableId", STRING))
                        .returns("LadderResult")
                        .doc("What a finished ranked table did to your rank: pips, rank-ups, and what reaching a rank paid. "
                                + "Null while it is still being played, or long after.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            UUID tableId = call.uuid(0);
                            RankedService.Result result = RankedService.get().result(tableId);
                            if (result == null) {
                                return null;
                            }
                            LadderStore.GameRow row = ladder(() -> RankedService.get().store().game(tableId.toString()));
                            if (row == null || !row.user.equalsIgnoreCase(user)) {
                                return null;
                            }
                            return ladder(() -> resultView(user, result, System.currentTimeMillis()));
                        })
        );
    }

    private static String format(String format) throws RpcException {
        if (!RankedService.isFormat(format)) {
            throw RpcException.invalidParams("There is no ranked ladder for " + format + ".");
        }
        return format;
    }

    /** a format with games to queue for: not Limited, whose rank moves with Career's sealed and draft runs */
    private static String queued(String format) throws RpcException {
        if (RankedService.LIMITED.equals(format)) {
            throw RpcException.invalidParams("The Limited rank moves with Career's sealed and draft runs: it has no queue.");
        }
        return format(format);
    }

    private static String queue(String queue) throws RpcException {
        if (!RankedService.isQueue(queue)) {
            throw RpcException.invalidParams("There is no " + queue + " queue.");
        }
        return queue;
    }

    interface LadderCall<T> {
        T run() throws SQLException;
    }

    static <T> T ladder(LadderCall<T> work) throws RpcException {
        try {
            return work.run();
        } catch (SQLException | IllegalStateException e) {
            logger.error("Ranked store failed", e);
            throw new RpcException(RpcException.SERVER_ERROR, "Ranked standings can't be read or saved right now", e);
        }
    }

    static LadderState state(String user, String format, long now) throws SQLException {
        return state(user, format, RankedService.OPEN, now);
    }

    static LadderState state(String user, String format, String queue, long now) throws SQLException {
        RankedService ranked = RankedService.get();
        String key = user.toLowerCase(java.util.Locale.ENGLISH);
        int season = ranked.season(now);
        ranked.rollover(key, format, queue, season, now);
        LadderStore.Standing standing = ranked.store().standing(key, format, queue, season);
        LadderState view = new LadderState();
        view.season = season;
        view.seasonName = mage.server.ladder.Seasons.name(season);
        view.seasonEnds = mage.server.ladder.Seasons.get().endsAt(Math.max(season, mage.server.ladder.Seasons.get().index(now)));
        view.seasonDaysLeft = (int) Math.max(1, (view.seasonEnds - now + 86_399_999L) / 86_400_000L);
        view.seasons = new ArrayList<>();
        for (LadderStore.Standing past : ranked.store().standings(key, format, queue)) {
            if (past.season >= season) {
                continue;
            }
            LadderPastSeason p = new LadderPastSeason();
            p.season = past.season;
            p.seasonName = mage.server.ladder.Seasons.name(past.season);
            p.peakStep = past.peak;
            p.peakRank = LadderRank.rankName(past.peak);
            p.peakLabel = LadderRank.label(past.peak);
            p.endLabel = LadderRank.label(past.step);
            if (view.seasons.isEmpty()) {
                p.placedLabel = LadderRank.label(LadderRank.placement(past.step));
            }
            view.seasons.add(p);
        }
        if (!view.seasons.isEmpty()) {
            LadderPastSeason last = view.seasons.get(0);
            view.lastSeason = last;
            if (RankedService.LEAGUE.equals(queue)) {
                CareerLeague.SeasonEnd end = CareerLeague.seasonEnd(last.peakStep);
                last.packs = end.packs;
                last.mythicWildcards = end.mythic;
            }
        }
        view.formats = ranked.formats();
        view.format = format;
        view.queue = queue;
        int step = standing == null ? 0 : standing.step;
        view.step = step;
        view.rank = LadderRank.rankName(step);
        view.tier = LadderRank.tier(step);
        view.pips = LadderRank.pips(step);
        view.pipsPerTier = LadderRank.PIPS;
        view.mythicStep = LadderRank.MYTHIC_STEP;
        view.label = LadderRank.label(step);
        view.peakStep = standing == null ? 0 : standing.peak;
        view.peakLabel = LadderRank.label(view.peakStep);
        if (standing != null) {
            view.streak = standing.streak;
            view.won = standing.won;
            view.lost = standing.lost;
            view.drew = standing.drew;
        }
        if (step >= LadderRank.MYTHIC_STEP && !RankedService.LIMITED.equals(format)) {
            view.mythicRating = (int) Math.round(ranked.ratings().player(user, format, queue, now).rating);
        }
        view.playing = ranked.playing(user);
        view.recent = new ArrayList<>();
        for (LadderStore.GameRow row : ranked.store().games(user.toLowerCase(java.util.Locale.ENGLISH), format, queue, RECENT)) {
            if ("void".equals(row.status)) {
                continue;
            }
            LadderGame game = new LadderGame();
            game.id = row.id;
            game.status = row.status;
            game.opponent = opponent(ranked.roster().bot(row.bot));
            game.deck = row.deck;
            game.stepBefore = row.stepBefore;
            game.stepAfter = row.stepAfter;
            game.started = row.started;
            game.ended = row.ended;
            view.recent.add(game);
        }
        return view;
    }

    static LadderOpponent opponent(BotRoster.Bot bot) {
        if (bot == null) {
            return null;
        }
        LadderOpponent view = new LadderOpponent();
        view.botId = bot.id;
        view.name = bot.persona == null ? bot.deckName : bot.persona.name;
        if (bot.persona != null) {
            view.avatar = bot.persona.avatar;
            view.sleeve = bot.persona.sleeve;
            view.playmat = bot.persona.playmat;
            view.frame = bot.persona.frame;
            view.title = bot.persona.title;
        }
        view.deckName = bot.deckName;
        view.colors = bot.colors;
        view.search = bot.search;
        return view;
    }

    private static LadderResult resultView(String user, RankedService.Result result, long now) throws SQLException {
        LadderResult view = new LadderResult();
        view.tableId = result.tableId;
        view.format = result.format;
        view.status = result.status;
        view.opponent = opponent(RankedService.get().roster().bot(result.botId));
        view.stepBefore = result.stepBefore;
        view.stepAfter = result.stepAfter;
        view.labelBefore = LadderRank.label(result.stepBefore);
        view.labelAfter = LadderRank.label(result.stepAfter);
        view.delta = result.delta;
        view.bonus = result.bonus;
        view.streak = result.streak;
        view.reached = result.reached;
        view.cosmetics = result.paid == null ? new ArrayList<>() : result.paid.cosmetics;
        view.tokens = result.paid == null ? 0 : result.paid.tokens;
        view.queue = result.queue;
        view.packs = result.paid == null ? 0 : result.paid.packs;
        view.wildcards = result.paid == null ? new java.util.LinkedHashMap<>() : result.paid.wildcards;
        if (result.stepAfter >= LadderRank.MYTHIC_STEP) {
            view.mythicRating = (int) Math.round(result.ratingAfter);
        }
        return view;
    }

    private static Object play(ApiContext ctx, RpcCall call) throws RpcException {
        String user = CareerApi.career(ctx, call);
        String format = queued(call.string(0));
        DeckCardLists deck = call.object(1, DeckCardLists.class);
        String queue = queue(call.optString(2, RankedService.OPEN));
        if (RankedService.LEAGUE.equals(queue)) {
            throw RpcException.invalidParams("League games start with leaguePlay and a League deck.");
        }
        return start(ctx, call, user, format, queue, deck);
    }

    /** a League game: the server's copy of the League deck, owned and legal */
    private static Object leaguePlay(ApiContext ctx, RpcCall call) throws RpcException {
        String user = CareerApi.career(ctx, call);
        String format = format(call.string(0));
        String deckId = call.string(1);
        if (RankedService.get().playing(user)) {
            throw RpcException.invalidParams("Finish your ranked game first: one at a time.");
        }
        CareerLeague.LeagueDeck deck = CareerApi.store(() -> CareerLeague.get().playable(user, format, deckId));
        return start(ctx, call, user, format, RankedService.LEAGUE, deck.deck);
    }

    private static Object start(ApiContext ctx, RpcCall call, String user, String format, String queue, DeckCardLists deck) throws RpcException {
        RankedService ranked = RankedService.get();
        if (ranked.playing(user)) {
            throw RpcException.invalidParams("Finish your ranked game first: one at a time.");
        }
        String deckType = RankedService.deckType(format);
        List<String> problems = legality(deckType, deck);
        if (!problems.isEmpty()) {
            String name = Character.toUpperCase(format.charAt(0)) + format.substring(1);
            throw RpcException.invalidParams("This deck isn't legal in " + name + ": " + String.join("; ", problems) + ".");
        }
        long now = System.currentTimeMillis();
        BotRoster.Bot bot = ladder(() -> ranked.pick(user, format, queue, now));
        DeckCardLists botDeck;
        try {
            botDeck = ranked.roster().deck(bot);
        } catch (java.io.IOException e) {
            throw new RpcException(RpcException.SERVER_ERROR, "The opponent's deck can't be read", e);
        }
        String name = bot.persona == null ? bot.deckName : bot.persona.name;
        String deckName = deck.getName();
        UUID[] registered = new UUID[1];
        String tableId;
        try {
            mage.server.career.CareerCampaigns.DuelSeat seat = new mage.server.career.CareerCampaigns.DuelSeat();
            seat.name = name;
            seat.skill = bot.skill;
            seat.aiType = null; // the Mad AI, which the Rust host plays
            seat.deck = botDeck;
            tableId = CareerApi.startMatch(ctx, call, user, "Two Player Duel", deckType, RankedService.winsNeeded(queue), deck,
                    java.util.Collections.singletonList(seat), id -> {
                        try {
                            ranked.register(id, user, format, queue, bot, deckName, now);
                            registered[0] = id;
                        } catch (SQLException e) {
                            throw new IllegalStateException("The ranked game couldn't be recorded", e);
                        }
                    });
        } catch (RpcException e) {
            if (registered[0] != null) {
                ranked.forget(registered[0], System.currentTimeMillis());
            }
            throw e;
        }
        LadderMatch match = new LadderMatch();
        match.tableId = tableId;
        match.opponent = opponent(bot);
        match.warm = CareerApi.printings(botDeck);
        return match;
    }

    /** why the deck isn't legal in the deck type (the first few reasons), or nothing */
    static List<String> legality(String deckType, DeckCardLists lists) throws RpcException {
        try {
            return DeckLegality.problems(deckType, lists);
        } catch (IllegalStateException e) {
            throw new RpcException(RpcException.SERVER_ERROR, e.getMessage());
        } catch (IllegalArgumentException e) {
            throw RpcException.invalidParams(e.getMessage());
        }
    }
}
