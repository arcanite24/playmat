package mage.server.websocket.api;

import mage.cards.decks.DeckCardLists;
import mage.constants.MultiplayerAttackOption;
import mage.constants.RangeOfInfluence;
import mage.game.match.MatchOptions;
import mage.players.PlayerType;
import mage.server.career.CareerCampaigns;
import mage.server.career.CareerLadder;
import mage.server.career.CareerContent;
import mage.server.career.CareerPayout;
import mage.server.career.CareerProfile;
import mage.server.career.CareerService;
import mage.server.career.CareerStore;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;
import mage.view.TableView;
import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.UUID;

import static mage.server.websocket.rpc.RpcParam.Type.BOOLEAN;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.object;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: the optional single-player Career. Everything is kept and decided on the server: the
 * collection, coins, which opponents are open, and what a match pays.
 */
final class CareerApi {

    private static final Logger logger = Logger.getLogger(CareerApi.class);

    private CareerApi() {
    }

    /** The Career screen's state: whether Career is offered, and the account's Career if it has one. */
    public static class CareerState {
        public boolean enabled;
        public CareerProfile profile;
        /** the latest payouts, newest first */
        public List<CareerPayout> recent;
        /** the ladder badge earned (S037), null before the first */
        public CareerLadder.CareerRank rank;
        /** the adaptive skill Career games play at when asked for (S049) */
        public mage.server.career.AdaptiveSkill.AdaptiveView adaptive;
    }

    /** A Career match was set up; the game starts like any other. */
    public static class CareerMatch {
        public String tableId;
        public String opponentId;
        /** the printings in the opponent's deck, so the client can fetch their pictures while the table is set */
        public List<CareerContent.CareerCover> warm;
    }

    /** each printing in a deck once (main and sideboard); empty without a deck */
    static List<CareerContent.CareerCover> printings(DeckCardLists deck) {
        List<CareerContent.CareerCover> printings = new ArrayList<>();
        if (deck == null) {
            return printings;
        }
        Set<String> seen = new HashSet<>();
        List<mage.cards.decks.DeckCardInfo> cards = new ArrayList<>(deck.getCards());
        cards.addAll(deck.getSideboard());
        for (mage.cards.decks.DeckCardInfo card : cards) {
            if (seen.add(card.getSetCode() + "/" + card.getCardNumber() + "/" + card.getCardName())) {
                CareerContent.CareerCover printing = new CareerContent.CareerCover();
                printing.name = card.getCardName();
                printing.setCode = card.getSetCode();
                printing.cardNumber = card.getCardNumber();
                printings.add(printing);
            }
        }
        return printings;
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("careerState")
                        .returns("CareerState")
                        .doc("Whether this server offers Career, and the signed-in account's Career (profile null before it opens one).")
                        .handler(call -> {
                            CareerState state = new CareerState();
                            state.enabled = enabled(ctx);
                            if (state.enabled) {
                                String user = user(ctx, call);
                                // progression starts with the account's first visit or first counted game (B264)
                                state.profile = store(() -> CareerService.get().profileOrNew(user));
                                state.recent = store(() -> CareerStore.get().payouts(user, 10));
                                if (state.profile != null) {
                                    state.rank = store(() -> CareerLadder.rank(user));
                                    state.adaptive = store(() -> mage.server.career.AdaptiveSkill.view(user, "career"));
                                }
                            }
                            return state;
                        }),

                RpcMethod.named("careerStarters")
                        .returns("CareerStarter[]")
                        .doc("The starter decks a Career can open with.")
                        .handler(call -> {
                            career(ctx, call);
                            return CareerContent.get().starters();
                        }),

                RpcMethod.named("careerStart")
                        .params(of("starterId", STRING))
                        .returns("CareerProfile")
                        .doc("Open a Career with a starter deck: its cards become the collection, plus coins for a first pack.")
                        .handler(call -> {
                            String user = career(ctx, call);
                            String starterId = call.string(0);
                            return store(() -> CareerService.get().start(user, starterId));
                        }),

                RpcMethod.named("careerDeck")
                        .returns("DeckCardLists | null")
                        .doc("The account's Career deck, kept on the server so it follows the account to any device. Null "
                                + "before one is saved.")
                        .handler(call -> {
                            String user = career(ctx, call);
                            return store(() -> CareerService.get().deck(user));
                        }),

                RpcMethod.named("careerSaveDeck")
                        .params(object("deck", "DeckCardLists"))
                        .returns("boolean")
                        .doc("Save the account's Career deck. Refused, with the cards it's short of, when the deck holds more "
                                + "copies of a card than the collection does (basic lands are free).")
                        .handler(call -> {
                            String user = career(ctx, call);
                            DeckCardLists deck = call.object(0, DeckCardLists.class);
                            return store(() -> {
                                CareerService.get().saveDeck(user, deck);
                                return true;
                            });
                        }),

                RpcMethod.named("careerCollection")
                        .returns("CareerCard[]")
                        .doc("Every card the account owns in Career, by printing. Basic lands are free and not listed.")
                        .handler(call -> {
                            String user = career(ctx, call);
                            return store(() -> CareerStore.get().collection(user));
                        }),

                RpcMethod.named("careerOpponents")
                        .returns("CareerOpponent[]")
                        .doc("The AI opponents in tiers, with the account's wins against each and which are open. A tier opens "
                                + "after enough wins in the one before.")
                        .handler(call -> {
                            String user = career(ctx, call);
                            return store(() -> CareerService.get().opponents(user));
                        }),

                RpcMethod.named("careerPlay")
                        .params(of("opponentId", STRING), object("deck", "DeckCardLists"), optional("adaptive", BOOLEAN))
                        .returns("CareerMatch")
                        .doc("Start a Career match against an open opponent with a deck of owned cards (basic lands are free). "
                                + "The server sets up the table and starts the match; the game arrives like any other. Adaptive: the "
                                + "AI's skill follows your hidden Career rating instead of the tier's, to keep your win rate near 55%.")
                        .handler(call -> play(ctx, call)),

                RpcMethod.named("careerShop")
                        .returns("CareerShopSet[]")
                        .doc("The sets the Career shop sells packs of, newest first, with the price in coins: the newest sets, and "
                                + "the ones campaign chapters opened. Sets a chapter still has to open come last, locked, saying which.")
                        .handler(call -> {
                            String user = career(ctx, call);
                            return store(() -> CareerService.get().shop(user));
                        }),

                RpcMethod.named("careerBuyPack")
                        .params(of("setCode", STRING))
                        .returns("CareerPackResult")
                        .doc("Buy and open a pack. Copies past four of a card become coins (commons, uncommons) or a wildcard "
                                + "(rares, mythics); those cards say so in convertedTo.")
                        .handler(call -> {
                            String user = career(ctx, call);
                            String setCode = call.string(0);
                            return store(() -> CareerService.get().buyPack(user, setCode));
                        }),

                RpcMethod.named("careerCraft")
                        .params(of("setCode", STRING), of("cardNumber", STRING))
                        .returns("CareerProfile")
                        .doc("Craft one copy of a card: commons and uncommons cost coins, rares and mythics a wildcard of their rarity.")
                        .handler(call -> {
                            String user = career(ctx, call);
                            String setCode = call.string(0);
                            String cardNumber = call.string(1);
                            return store(() -> CareerService.get().craft(user, setCode, cardNumber));
                        }),

                RpcMethod.named("careerExport")
                        .returns("string")
                        .doc("The account's whole Career as a signed text file, to keep a copy.")
                        .handler(call -> {
                            String user = career(ctx, call);
                            return store(() -> CareerService.get().export(user));
                        }),

                RpcMethod.named("careerImport")
                        .params(of("data", STRING))
                        .returns("CareerProfile")
                        .doc("Bring back an exported Career, for an account that has none. Only files this server made for this "
                                + "account are taken.")
                        .handler(call -> {
                            String user = career(ctx, call);
                            String data = call.string(0);
                            return store(() -> CareerService.get().importFile(user, data));
                        })
        );
    }

    static boolean enabled(ApiContext ctx) {
        return ctx.managers != null && CareerService.enabled(ctx.managers.configSettings().isAuthenticationActivated());
    }

    static String user(ApiContext ctx, RpcCall call) throws RpcException {
        return ctx.sessions.userName(call.sessionId()).orElseThrow(() -> RpcException.notAuthorized("Sign in first"));
    }

    /** the signed-in account, when this server offers Career */
    static String career(ApiContext ctx, RpcCall call) throws RpcException {
        String user = user(ctx, call);
        if (!enabled(ctx)) {
            throw RpcException.notAuthorized("This server doesn't offer Career");
        }
        return user;
    }

    interface StoreCall<T> {
        T run() throws SQLException, CareerService.CareerException;
    }

    static <T> T store(StoreCall<T> work) throws RpcException {
        try {
            return work.run();
        } catch (CareerService.CareerException e) {
            throw RpcException.invalidParams(e.getMessage());
        } catch (SQLException | IllegalStateException e) {
            logger.error("Career store failed", e);
            throw new RpcException(RpcException.SERVER_ERROR, "Career progress can't be read or saved right now", e);
        }
    }

    private static Object play(ApiContext ctx, RpcCall call) throws RpcException {
        String user = career(ctx, call);
        String opponentId = call.string(0);
        DeckCardLists deck = call.object(1, DeckCardLists.class);
        CareerContent content = CareerContent.get();
        CareerContent.Opponent opponent = content.opponent(opponentId);
        if (opponent == null) {
            throw RpcException.invalidParams("There is no opponent " + opponentId + ".");
        }
        if (store(() -> CareerStore.get().profile(user)) == null) {
            throw RpcException.invalidParams("Open a Career first.");
        }
        if (!store(() -> CareerService.get().unlocked(user, opponentId))) {
            throw RpcException.invalidParams(opponent.name + " isn't open yet: win more games in the tier before.");
        }
        int size = deck.getCards().stream().mapToInt(card -> card.getAmount()).sum();
        if (size < 60) {
            throw RpcException.invalidParams("A Career deck needs at least 60 cards; this one has " + size + ".");
        }
        List<String> problems = store(() -> CareerService.get().ownershipProblems(user, deck));
        if (!problems.isEmpty()) {
            throw RpcException.invalidParams("You don't own enough copies: " + String.join("; ", problems.subList(0, Math.min(5, problems.size())))
                    + (problems.size() > 5 ? " (and " + (problems.size() - 5) + " more)" : "") + ".");
        }
        DeckCardLists opponentDeck;
        long seed = store(() -> mage.server.career.CareerLadder.deckSeed(user, opponent.id));
        try {
            // the opponent's list rotates by seed (S038)
            opponentDeck = content.opponentDeck(opponent, seed);
        } catch (java.io.IOException e) {
            throw new RpcException(RpcException.SERVER_ERROR, "The opponent's deck can't be read", e);
        }

        CareerContent.Tier tier = content.tier(opponent.tier);
        boolean adaptive = call.has(2) && call.bool(2);
        int skill = adaptive ? store(() -> mage.server.career.AdaptiveSkill.skillFor(user, "career"))
                : opponent.skill != null ? opponent.skill : tier.skill;
        PlayerType aiType = aiType(opponent.aiType != null ? opponent.aiType : tier.aiType);
        mage.server.career.CareerSetup setup = store(() -> mage.server.career.CareerLadder.setup(user, opponent));
        String tableId = startMatch(ctx, call, user, opponent.name, "Constructed - Freeform", deck, aiType, skill, opponentDeck,
                id -> {
                    CareerService.get().register(id, user, opponent.id, opponent.tier, setup);
                    if (adaptive) {
                        mage.server.career.AdaptiveSkill.mark(id, user, "career");
                    }
                });
        CareerMatch match = new CareerMatch();
        match.tableId = tableId;
        match.opponentId = opponent.id;
        match.warm = printings(opponentDeck);
        return match;
    }

    /**
     * Sets up and starts a Career table: the player against one AI, best of one, no rollbacks (a rollback could
     * replay a lost game for its payout). {@code register} tells the Career service about the table before anyone
     * sits, so the game starts with the right setup.
     *
     * @return the table id
     */
    static String startMatch(ApiContext ctx, RpcCall call, String user, String opponentName, String deckType, DeckCardLists deck,
                             PlayerType aiType, int skill, DeckCardLists opponentDeck, java.util.function.Consumer<UUID> register) throws RpcException {
        CareerCampaigns.DuelSeat seat = new CareerCampaigns.DuelSeat();
        seat.name = opponentName;
        seat.skill = skill;
        seat.aiType = aiType == null ? null : aiType.name();
        seat.deck = opponentDeck;
        return startMatch(ctx, call, user, "Two Player Duel", deckType, 1, deck, Collections.singletonList(seat), register);
    }

    /**
     * Sets up and starts a Career table of any game type: the player against one AI or more (a pod), best of
     * {@code winsNeeded * 2 - 1}. Every deck is checked against {@code deckType} as it sits down.
     *
     * @return the table id
     */
    static String startMatch(ApiContext ctx, RpcCall call, String user, String gameType, String deckType, int winsNeeded,
                             DeckCardLists deck, List<CareerCampaigns.DuelSeat> seats, java.util.function.Consumer<UUID> register) throws RpcException {
        String sessionId = call.sessionId();
        UUID roomId = ctx.managers.gamesRoomManager().getMainRoomId();
        boolean pod = seats.size() > 1;
        MatchOptions options = new MatchOptions(user + " vs " + seats.get(0).name + (pod ? " and " + (seats.size() - 1) + " more" : ""),
                gameType, pod);
        options.setDeckType(deckType);
        options.setWinsNeeded(Math.max(1, winsNeeded));
        options.setRollbackTurnsAllowed(false);
        options.setSpectatorsAllowed(true);
        if (pod) {
            // a free-for-all: anyone may attack anyone
            options.setAttackOption(MultiplayerAttackOption.MULTIPLE);
            options.setRange(RangeOfInfluence.ALL);
        }
        // a solo game against the AI: the player's quit ratio from other matches doesn't bar it (the default 0 refused anyone who ever left one)
        options.setQuitRatio(100);
        // the table's seats: without them nobody can join
        options.getPlayerTypes().add(PlayerType.HUMAN);
        List<PlayerType> types = new ArrayList<>();
        for (CareerCampaigns.DuelSeat seat : seats) {
            PlayerType ai = aiType(seat.aiType);
            types.add(ai);
            options.getPlayerTypes().add(ai);
        }
        UUID tableId = null;
        try {
            TableView table = ctx.server.roomCreateTable(sessionId, roomId, options);
            tableId = table.getTableId();
            register.accept(tableId);
            if (!ctx.server.roomJoinTable(sessionId, roomId, tableId, user, PlayerType.HUMAN, 1, deck, "")) {
                throw RpcException.invalidParams("Your deck wasn't accepted for the table.");
            }
            Set<String> names = new HashSet<>();
            names.add(user);
            for (int i = 0; i < seats.size(); i++) {
                CareerCampaigns.DuelSeat seat = seats.get(i);
                String name = CareerContent.aiSeatName(seat.name);
                // two seats can't share a name
                for (int n = 2; !names.add(name); n++) {
                    name = CareerContent.aiSeatName(seat.name + " " + n);
                }
                if (!ctx.server.roomJoinTable(sessionId, roomId, tableId, name, types.get(i), Math.max(1, seat.skill), seat.deck, "")) {
                    throw new RpcException(RpcException.SERVER_ERROR, seat.name + " couldn't take a seat.");
                }
            }
            if (!ctx.server.matchStart(sessionId, roomId, tableId)) {
                throw new RpcException(RpcException.SERVER_ERROR, "The match couldn't start.");
            }
        } catch (RpcException e) {
            abandon(ctx, sessionId, roomId, tableId);
            throw e;
        } catch (Exception e) {
            abandon(ctx, sessionId, roomId, tableId);
            throw new RpcException(RpcException.SERVER_ERROR, "The Career match couldn't be set up: " + e.getMessage(), e);
        }
        return tableId.toString();
    }

    static PlayerType aiType(String name) {
        if (name == null) {
            return PlayerType.COMPUTER_MAD;
        }
        if (mage.server.career.Archmage.AI_TYPE.equals(name)) {
            return mage.server.career.Archmage.seat();
        }
        try {
            return PlayerType.valueOf(name);
        } catch (IllegalArgumentException e) {
            return PlayerType.COMPUTER_MAD;
        }
    }

    private static void abandon(ApiContext ctx, String sessionId, UUID roomId, UUID tableId) {
        if (tableId == null) {
            return;
        }
        CareerService.get().forget(tableId);
        try {
            ctx.server.tableRemove(sessionId, roomId, tableId);
        } catch (Exception e) {
            logger.warn("Career: an unfinished table couldn't be removed", e);
        }
    }
}
