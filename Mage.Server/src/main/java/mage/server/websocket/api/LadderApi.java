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

import static mage.server.websocket.rpc.RpcParam.Type.BOOLEAN;
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
        /** WUBRG letters from the deck's spells and lands */
        public String colors;
        /** how hard it searches: quick, light, half or full */
        public String search;
        /** macro archetype: aggro, burn, tempo, midrange, control, ramp, combo, graveyard, sacrifice, tribal */
        public String archetype;
        /** power tier: starter, budget, rogue, tier2, meta */
        public String tier;
        public String tagline;
        public List<String> keyCards;
        /** true on pre-match preview when exact decklist is kept hidden until the game starts */
        public boolean deckHidden;
        public Integer vsWon;
        public Integer vsLost;
    }

    /** A ranked game, as listed */
    public static class LadderGame {
        public String id;
        /** playing, won, lost, drew or void */
        public String status;
        public LadderOpponent opponent;
        public String deck;
        public String botArchetype;
        public Integer stepBefore;
        public Integer stepAfter;
        public Integer delta;
        public Integer turns;
        public String gameScore;
        public boolean inspectable;
        public long started;
        public Long ended;
    }

    /** One archetype slice in the player's current rating band */
    public static class LadderFieldEntry {
        public String archetype;
        public int decks;
        public List<String> sampleDecks = new ArrayList<>();
        public int won;
        public int lost;
    }

    /** Player's seasonal performance with one of their own decks */
    public static class LadderDeckStat {
        public String deck;
        public int won;
        public int lost;
        public int drew;
    }

    /** Player's seasonal matchup record against one opponent archetype */
    public static class LadderArchetypeStat {
        public String archetype;
        public int won;
        public int lost;
        public int drew;
    }

    /** A seasonal ranked milestone contract */
    public static class LadderContract {
        public String id;
        public int progress;
        public int target;
        public boolean done;
    }

    /** One card line of an inspected post-match opponent decklist */
    public static class LadderDeckCard {
        public int amount;
        public String name;
    }

    /** Opponent's full decklist, inspectable only after a ranked match finishes */
    public static class LadderOpponentDeck {
        public String gameId;
        public String botId;
        public String opponentName;
        public String deckName;
        public String archetype;
        public String colors;
        public String tagline;
        public List<LadderDeckCard> main = new ArrayList<>();
        public List<LadderDeckCard> sideboard = new ArrayList<>();
    }

    /** One row on the Living Mythic Standings leaderboard (humans and/or top calibrated bots) */
    public static class LadderMythicRow {
        public int rank;
        public String name;
        public int rating;
        public boolean bot;
        public String deckName;
        public String archetype;
        public String colors;
        public int games;
        public boolean you;
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
        /** true when one win reaches the next rank */
        public boolean promotionNext;
        /** the next rank name when promotionNext is true */
        public String promotionTarget;
        /** the lowest label a single loss can't drop the player below */
        public String floorLabel;
        /** next unpaid rank reward target (silver .. mythic), or null at Mythic */
        public String nextRewardRank;
        /** whether reaching nextRewardRank also pays a capsule token */
        public boolean nextRewardToken;
        /** active archetypes in the player's current rating band */
        public List<LadderFieldEntry> fieldMeta;
        /** player's seasonal W-L-D per deck piloted */
        public List<LadderDeckStat> deckStats;
        /** player's seasonal W-L-D vs opponent archetypes */
        public List<LadderArchetypeStat> archetypeStats;
        /** seasonal ranked milestone contracts */
        public List<LadderContract> contracts;
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
        /** all 3 AI opponents in a 4-player Commander pod */
        public List<LadderOpponent> podOpponents;
        public List<CareerContent.CareerCover> warm;
    }

    /** What a ranked game did, for the screen after it */
    public static class LadderResult {
        public String tableId;
        public String format;
        /** won, lost or drew */
        public String status;
        public LadderOpponent opponent;
        /** 1..4 finish placement in a 4-player Commander pod, or null for 1v1 */
        public Integer placement;
        /** all 3 pod opponents when played in a 4-player Commander pod */
        public List<LadderOpponent> podOpponents;
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
                                + "until it's played, so it can't be skipped, and hides exact deck details until the match starts. "
                                + "Queue: open (the default), bo3 or league.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String format = queued(call.string(0));
                            String queue = queue(call.optString(1, RankedService.OPEN));
                            return ladder(() -> blindOpponent(user, format, queue,
                                    RankedService.get().pick(user, format, queue, System.currentTimeMillis())));
                        }),

                RpcMethod.named("ladderPlay")
                        .params(of("format", STRING), object("deck", "DeckCardLists"), optional("queue", STRING))
                        .returns("LadderMatch")
                        .doc("Start a ranked game in the format with a deck legal there, against the picked bot. Queue: open "
                                + "(Bo1, the default) or bo3 (best of three; the sideboard comes into play between games). No hints, "
                                + "take-backs, practice tools or saves; conceding or leaving is a loss.")
                        .handler(call -> play(ctx, call)),

                RpcMethod.named("leaguePlay")
                        .params(of("format", STRING), of("deckId", STRING), optional("pod", BOOLEAN))
                        .returns("LadderMatch")
                        .doc("Start a League game in the format with one of your League decks (by id), against the League's picked "
                                + "bot (or a 4-player pod when format is commander and pod is true). The deck must be legal there and every card in it owned.")
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
                        }),

                RpcMethod.named("ladderGameDeck")
                        .params(of("gameId", STRING))
                        .returns("LadderOpponentDeck")
                        .doc("Inspect the opponent's full 60+15 decklist for one of your finished ranked games. Returns null "
                                + "while the game is still being played or for another player's game.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String gameId = call.string(0);
                            return ladder(() -> gameDeck(user, gameId));
                        }),

                RpcMethod.named("ladderMythicStandings")
                        .params(of("format", STRING), optional("queue", STRING), optional("includeBots", BOOLEAN))
                        .returns("LadderMythicRow[]")
                        .doc("Living Mythic Standings for a format and queue: ranks Mythic human players alongside the top "
                                + "calibrated ladder bots by live Glicko-2 rating (set includeBots=false for humans only).")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String format = queued(call.string(0));
                            String queue = queue(call.optString(1, RankedService.OPEN));
                            boolean includeBots = call.optBool(2, true);
                            return ladder(() -> mythicStandings(user, format, queue, includeBots, System.currentTimeMillis()));
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
        // Rank floor, promotion stakes, and next rank reward preview
        int rankIdx = LadderRank.rank(step);
        if (step < LadderRank.MYTHIC_STEP) {
            int nextRankStart = (rankIdx + 1) * LadderRank.RANK_STEPS;
            view.promotionNext = step + 1 >= nextRankStart;
            view.promotionTarget = LadderRank.RANKS[rankIdx + 1];
            view.nextRewardRank = LadderRank.RANKS[rankIdx + 1];
            view.nextRewardToken = "gold".equals(view.nextRewardRank)
                    || "diamond".equals(view.nextRewardRank)
                    || "mythic".equals(view.nextRewardRank);
        }
        if (step >= LadderRank.MYTHIC_STEP) {
            view.floorLabel = "Mythic";
        } else if (rankIdx == 0) {
            view.floorLabel = LadderRank.label(step);
        } else if (rankIdx == 1) {
            view.floorLabel = LadderRank.label(step - LadderRank.pips(step));
        } else {
            view.floorLabel = LadderRank.label(rankIdx * LadderRank.RANK_STEPS);
        }

        // Seasonal analytics: deck W-L, archetype matchups, contracts, and field meta forecast
        List<LadderStore.GameRow> seasonGames = ranked.store().seasonGames(key, format, queue, season);
        java.util.Map<String, LadderDeckStat> decksMap = new java.util.LinkedHashMap<>();
        java.util.Map<String, LadderArchetypeStat> archMap = new java.util.LinkedHashMap<>();
        java.util.Set<String> wonDecks = new java.util.HashSet<>();
        java.util.Set<String> defeatedArchetypes = new java.util.HashSet<>();
        java.util.Set<String> lostArchetypes = new java.util.HashSet<>();
        boolean avengedRival = false;
        boolean bo3Comeback = false;
        int bestStreak = 0;
        int curStreak = 0;
        // Iterate oldest-to-newest for streak and rival-avenged tracking
        for (int i = seasonGames.size() - 1; i >= 0; i--) {
            LadderStore.GameRow g = seasonGames.get(i);
            BotRoster.Bot met = ranked.roster().bot(g.bot);
            String arch = g.botArchetype != null && !g.botArchetype.isEmpty()
                    ? g.botArchetype
                    : (met != null ? met.effectiveArchetype() : "midrange");
            String deckLabel = g.deck != null && !g.deck.isEmpty() ? g.deck : "Deck";

            LadderDeckStat ds = decksMap.computeIfAbsent(deckLabel, d -> {
                LadderDeckStat s = new LadderDeckStat();
                s.deck = d;
                return s;
            });
            LadderArchetypeStat as = archMap.computeIfAbsent(arch, a -> {
                LadderArchetypeStat s = new LadderArchetypeStat();
                s.archetype = a;
                return s;
            });
            if ("won".equals(g.status)) {
                ds.won++;
                as.won++;
                wonDecks.add(deckLabel);
                defeatedArchetypes.add(arch);
                if (lostArchetypes.contains(arch)) {
                    avengedRival = true;
                }
                if ("2-1".equals(g.gameScore)) {
                    bo3Comeback = true;
                }
                curStreak++;
                if (curStreak > bestStreak) {
                    bestStreak = curStreak;
                }
            } else if ("lost".equals(g.status)) {
                ds.lost++;
                as.lost++;
                lostArchetypes.add(arch);
                curStreak = 0;
            } else {
                ds.drew++;
                as.drew++;
                curStreak = 0;
            }
        }
        view.deckStats = new ArrayList<>(decksMap.values());
        view.deckStats.sort((a, b) -> Integer.compare((b.won + b.lost + b.drew), (a.won + a.lost + a.drew)));
        view.archetypeStats = new ArrayList<>(archMap.values());
        view.archetypeStats.sort((a, b) -> Integer.compare((b.won + b.lost + b.drew), (a.won + a.lost + a.drew)));
        view.contracts = buildContracts(wonDecks.size(), defeatedArchetypes.size(), bestStreak, avengedRival, bo3Comeback, queue);

        view.fieldMeta = new ArrayList<>();
        if (!RankedService.LIMITED.equals(format)) {
            double rating = ranked.ratings().player(user, format, queue, now).rating;
            List<mage.server.ladder.LadderRatings.Candidate> near = ranked.ratings().candidatesByDeck(format, queue, rating, 0, 14);
            java.util.Map<String, LadderFieldEntry> fieldMap = new java.util.LinkedHashMap<>();
            for (mage.server.ladder.LadderRatings.Candidate c : near) {
                String arch = c.bot.effectiveArchetype();
                LadderFieldEntry fe = fieldMap.computeIfAbsent(arch, a -> {
                    LadderFieldEntry e = new LadderFieldEntry();
                    e.archetype = a;
                    LadderArchetypeStat st = archMap.get(a);
                    if (st != null) {
                        e.won = st.won;
                        e.lost = st.lost;
                    }
                    return e;
                });
                fe.decks++;
                if (fe.sampleDecks.size() < 3 && c.bot.deckName != null && !fe.sampleDecks.contains(c.bot.deckName)) {
                    fe.sampleDecks.add(c.bot.deckName);
                }
            }
            view.fieldMeta.addAll(fieldMap.values());
            view.fieldMeta.sort((a, b) -> Integer.compare(b.decks, a.decks));
            if (view.fieldMeta.size() > 6) {
                view.fieldMeta = new ArrayList<>(view.fieldMeta.subList(0, 6));
            }
        }

        view.recent = new ArrayList<>();
        for (LadderStore.GameRow row : ranked.store().games(key, format, queue, RECENT)) {
            if ("void".equals(row.status)) {
                continue;
            }
            LadderGame game = new LadderGame();
            game.id = row.id;
            game.status = row.status;
            BotRoster.Bot met = ranked.roster().bot(row.bot);
            game.opponent = opponent(met);
            game.deck = row.deck;
            game.botArchetype = row.botArchetype != null ? row.botArchetype : (met != null ? met.effectiveArchetype() : null);
            game.stepBefore = row.stepBefore;
            game.stepAfter = row.stepAfter;
            if (row.stepBefore != null && row.stepAfter != null) {
                game.delta = row.stepAfter - row.stepBefore;
            }
            game.turns = row.turns;
            game.gameScore = row.gameScore;
            game.inspectable = met != null && !"playing".equals(row.status);
            game.started = row.started;
            game.ended = row.ended;
            view.recent.add(game);
        }
        return view;
    }

    private static List<LadderContract> buildContracts(int wonDecks, int defeatedArch, int bestStreak,
                                                       boolean avengedRival, boolean bo3Comeback, String queue) {
        List<LadderContract> list = new ArrayList<>();
        list.add(contract("versatile", wonDecks, 3));
        list.add(contract("field-tested", defeatedArch, 5));
        list.add(contract("streak-master", bestStreak, 5));
        list.add(contract("rival-avenged", avengedRival ? 1 : 0, 1));
        if (RankedService.BO3.equals(queue)) {
            list.add(contract("bo3-comeback", bo3Comeback ? 1 : 0, 1));
        }
        return list;
    }

    private static LadderContract contract(String id, int progress, int target) {
        LadderContract c = new LadderContract();
        c.id = id;
        c.progress = Math.min(progress, target);
        c.target = target;
        c.done = progress >= target;
        return c;
    }

    static LadderOpponent blindOpponent(String user, String format, String queue, BotRoster.Bot bot) throws SQLException {
        LadderOpponent view = opponent(bot);
        if (view == null) {
            return null;
        }
        // Compute head-to-head against this bot's deck
        int won = 0;
        int lost = 0;
        String key = user.toLowerCase(java.util.Locale.ENGLISH);
        for (LadderStore.GameRow g : RankedService.get().store().finishedGames(key, format, queue, 100)) {
            boolean same = bot.id.equals(g.bot) || (bot.deck != null && bot.deck.equals(g.botDeck));
            if (same) {
                if ("won".equals(g.status)) {
                    won++;
                } else if ("lost".equals(g.status)) {
                    lost++;
                }
            }
        }
        view.vsWon = won;
        view.vsLost = lost;
        // True Ladder: hide exact decklist and colors before the match starts so players cannot counter-queue
        view.deckHidden = true;
        view.deckName = null;
        view.colors = null;
        view.keyCards = null;
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
        view.archetype = bot.effectiveArchetype();
        view.tier = bot.effectiveTier();
        view.tagline = bot.tagline;
        view.keyCards = bot.keyCards;
        return view;
    }

    private static LadderOpponentDeck gameDeck(String user, String gameId) throws SQLException {
        RankedService ranked = RankedService.get();
        LadderStore.GameRow row = ranked.store().game(gameId);
        if (row == null || !row.user.equalsIgnoreCase(user) || "playing".equals(row.status) || "void".equals(row.status)) {
            return null;
        }
        BotRoster.Bot bot = ranked.roster().bot(row.bot);
        if (bot == null) {
            return null;
        }
        DeckCardLists lists;
        try {
            lists = ranked.roster().deck(bot);
        } catch (java.io.IOException e) {
            logger.warn("Ranked: couldn't read bot deck " + bot.deck + ": " + e.getMessage());
            return null;
        }
        LadderOpponentDeck view = new LadderOpponentDeck();
        view.gameId = gameId;
        view.botId = bot.id;
        view.opponentName = bot.persona == null ? bot.deckName : bot.persona.name;
        view.deckName = bot.deckName;
        view.archetype = bot.effectiveArchetype();
        view.colors = bot.colors;
        view.tagline = bot.tagline;
        view.main = summarizeCards(lists.getCards());
        view.sideboard = summarizeCards(lists.getSideboard());
        return view;
    }

    private static List<LadderDeckCard> summarizeCards(List<mage.cards.decks.DeckCardInfo> cards) {
        java.util.Map<String, Integer> counts = new java.util.LinkedHashMap<>();
        if (cards != null) {
            for (mage.cards.decks.DeckCardInfo c : cards) {
                counts.merge(c.getCardName(), 1, Integer::sum);
            }
        }
        List<LadderDeckCard> out = new ArrayList<>();
        for (java.util.Map.Entry<String, Integer> e : counts.entrySet()) {
            LadderDeckCard line = new LadderDeckCard();
            line.name = e.getKey();
            line.amount = e.getValue();
            out.add(line);
        }
        out.sort((a, b) -> Integer.compare(b.amount, a.amount) != 0
                ? Integer.compare(b.amount, a.amount)
                : a.name.compareTo(b.name));
        return out;
    }

    private static List<LadderMythicRow> mythicStandings(String user, String format, String queue, boolean includeBots, long now) throws SQLException {
        RankedService ranked = RankedService.get();
        int season = ranked.season(now);
        String userKey = user.toLowerCase(java.util.Locale.ENGLISH);
        List<LadderMythicRow> rows = new ArrayList<>();
        // Human Mythic players in the current season
        for (LadderStore.BoardRow br : ranked.store().mythicBoard(format, queue, season)) {
            LadderMythicRow r = new LadderMythicRow();
            r.name = br.user;
            r.rating = (int) br.value;
            r.bot = false;
            r.games = br.games;
            r.you = br.user.equalsIgnoreCase(userKey);
            rows.add(r);
        }
        // Top calibrated bots (deduplicated by deck, highest-rated copy per deck)
        if (includeBots) {
            java.util.Map<String, LadderMythicRow> bestByDeck = new java.util.LinkedHashMap<>();
            for (BotRoster.Bot bot : ranked.roster().bots(format, queue)) {
                double rating = ranked.ratings().bot(bot, queue).rating;
                String deckKey = bot.deck != null ? bot.deck : bot.id;
                LadderMythicRow prev = bestByDeck.get(deckKey);
                if (prev == null || rating > prev.rating) {
                    LadderMythicRow r = new LadderMythicRow();
                    r.name = bot.persona == null ? bot.deckName : bot.persona.name;
                    r.rating = (int) Math.round(rating);
                    r.bot = true;
                    r.deckName = bot.deckName;
                    r.archetype = bot.effectiveArchetype();
                    r.colors = bot.colors;
                    r.games = bot.games == null ? 0 : bot.games;
                    bestByDeck.put(deckKey, r);
                }
            }
            List<LadderMythicRow> botRows = new ArrayList<>(bestByDeck.values());
            botRows.sort((a, b) -> Integer.compare(b.rating, a.rating));
            if (botRows.size() > 25) {
                botRows = botRows.subList(0, 25);
            }
            rows.addAll(botRows);
        }
        rows.sort((a, b) -> Integer.compare(b.rating, a.rating) != 0
                ? Integer.compare(b.rating, a.rating)
                : Boolean.compare(a.bot, b.bot));
        for (int i = 0; i < rows.size(); i++) {
            rows.get(i).rank = i + 1;
        }
        return rows;
    }

    private static LadderResult resultView(String user, RankedService.Result result, long now) throws SQLException {
        LadderResult view = new LadderResult();
        view.tableId = result.tableId;
        view.format = result.format;
        view.status = result.status;
        view.opponent = opponent(RankedService.get().roster().bot(result.botId));
        view.placement = result.placement;
        if (result.podBotIds != null && !result.podBotIds.isEmpty()) {
            view.podOpponents = new ArrayList<>();
            for (String podBotId : result.podBotIds) {
                LadderOpponent opp = opponent(RankedService.get().roster().bot(podBotId));
                if (opp != null) {
                    view.podOpponents.add(opp);
                }
            }
        }
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
        if (RankedService.POD.equals(queue) && !"commander".equals(format)) {
            throw RpcException.invalidParams("The 4-player Pod queue is only available for Commander.");
        }
        return start(ctx, call, user, format, queue, deck, RankedService.POD.equals(queue));
    }

    /** a League game: the server's copy of the League deck, owned and legal */
    private static Object leaguePlay(ApiContext ctx, RpcCall call) throws RpcException {
        String user = CareerApi.career(ctx, call);
        String format = format(call.string(0));
        String deckId = call.string(1);
        boolean pod = call.optBool(2, false) && "commander".equals(format);
        if (RankedService.get().playing(user)) {
            throw RpcException.invalidParams("Finish your ranked game first: one at a time.");
        }
        CareerLeague.LeagueDeck deck = CareerApi.store(() -> CareerLeague.get().playable(user, format, deckId));
        return start(ctx, call, user, format, RankedService.LEAGUE, deck.deck, pod);
    }

    private static Object start(ApiContext ctx, RpcCall call, String user, String format, String queue, DeckCardLists deck, boolean pod) throws RpcException {
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
        boolean isCommander = "commander".equals(format);
        boolean isBrawl = "brawl".equals(format);
        boolean javaOnly = isCommander || isBrawl;
        List<BotRoster.Bot> bots = pod
                ? ladder(() -> ranked.pickPod(user, format, queue, now))
                : java.util.Collections.singletonList(ladder(() -> ranked.pick(user, format, queue, now)));
        BotRoster.Bot primary = bots.get(0);
        List<mage.server.career.CareerCampaigns.DuelSeat> seats = new ArrayList<>();
        List<CareerContent.CareerCover> warm = new ArrayList<>();
        List<LadderOpponent> podOpponents = new ArrayList<>();
        for (BotRoster.Bot bot : bots) {
            DeckCardLists botDeck;
            try {
                botDeck = ranked.roster().deck(bot);
            } catch (java.io.IOException e) {
                throw new RpcException(RpcException.SERVER_ERROR, "The opponent's deck can't be read", e);
            }
            mage.server.career.CareerCampaigns.DuelSeat seat = new mage.server.career.CareerCampaigns.DuelSeat();
            seat.name = bot.persona == null ? bot.deckName : bot.persona.name;
            seat.skill = bot.skill;
            // Commander and Brawl stay on Java AI (CLAUDE.md); 1v1 Constructed uses default Mad/Rust host routing
            seat.aiType = javaOnly ? PlayerType.COMPUTER_MAD.name() : null;
            seat.deck = botDeck;
            seats.add(seat);
            warm.addAll(CareerApi.printings(botDeck));
            podOpponents.add(opponent(bot));
        }
        String deckName = deck.getName();
        String gameType = isCommander
                ? (pod ? "Commander Free For All" : "Commander Two Player Duel")
                : isBrawl
                    ? "Brawl Two Player Duel"
                    : "Two Player Duel";
        UUID[] registered = new UUID[1];
        String tableId;
        try {
            tableId = CareerApi.startMatch(ctx, call, user, gameType, deckType, RankedService.winsNeeded(queue), deck,
                    seats, id -> {
                        try {
                            if (pod) {
                                ranked.registerPod(id, user, format, queue, bots, deckName, now);
                            } else {
                                ranked.register(id, user, format, queue, primary, deckName, now);
                            }
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
        match.opponent = opponent(primary);
        if (pod) {
            match.podOpponents = podOpponents;
        }
        match.warm = warm;
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
