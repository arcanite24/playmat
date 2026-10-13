package mage.server.websocket.api;

import mage.cards.decks.DeckCardLists;
import mage.players.PlayerType;
import mage.server.career.CareerCampaigns;
import mage.server.career.CareerChallenges;
import mage.server.career.CareerGauntlet;
import mage.server.career.CareerLimited;
import mage.server.career.CareerPuzzles;
import mage.server.career.CareerService;
import mage.server.career.CareerSetup;
import mage.server.career.CareerStore;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;

import java.io.IOException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import static mage.server.websocket.rpc.RpcParam.Type.ARRAY;
import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.object;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optionalObject;

/**
 * Web client bridge: Career's single-player modes beyond the roster. Campaigns, the puzzle book, the weekly
 * challenge, gauntlet runs and solo sealed and draft. Every game is set up and paid by the server; each play call
 * returns the table, and the game arrives like any other.
 */
final class CareerModesApi {

    private CareerModesApi() {
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                // ---- campaigns
                RpcMethod.named("careerCampaigns")
                        .returns("CareerCampaign[]")
                        .doc("Every campaign with its chapters and nodes, each locked, open or done for this account.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerCampaigns.get().campaigns(user));
                        }),
                RpcMethod.named("careerCampaignPlay")
                        .params(of("campaignId", STRING), of("nodeId", STRING))
                        .returns("CareerMatch")
                        .doc("Play a campaign duel or trial that's open (or done, to play it again), with the node's deck or the "
                                + "chapter's, under the school's format: a pod seats every AI, a final is a best of three. A first "
                                + "win pays the node's reward.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            String campaignId = call.string(0);
                            String nodeId = call.string(1);
                            CareerCampaigns.DuelPlan plan = plan(() -> CareerCampaigns.get().plan(user, campaignId, nodeId));
                            String tableId = CareerApi.startMatch(ctx, call, user, plan.gameType, plan.deckType, plan.winsNeeded,
                                    plan.playerDeck, plan.seats,
                                    id -> CareerService.get().register(id, user, plan.kind, plan.key, plan.setup));
                            DeckCardLists warm = new DeckCardLists();
                            for (CareerCampaigns.DuelSeat seat : plan.seats) {
                                warm.getCards().addAll(seat.deck.getCards());
                                warm.getSideboard().addAll(seat.deck.getSideboard());
                            }
                            return match(tableId, "campaign:" + plan.key, "trial".equals(plan.kind) ? null : warm);
                        }),
                RpcMethod.named("careerCampaignDeck")
                        .params(of("campaignId", STRING), of("nodeId", STRING))
                        .returns("DeckCardLists")
                        .doc("The list of the deck you play at a campaign duel, to read before it.")
                        .handler(call -> {
                            CareerApi.career(ctx, call);
                            String campaignId = call.string(0);
                            String nodeId = call.string(1);
                            return plan(() -> CareerCampaigns.get().playerDeck(campaignId, nodeId));
                        }),
                RpcMethod.named("careerCampaignChoose")
                        .params(of("campaignId", STRING), of("nodeId", STRING), of("optionId", STRING))
                        .returns("CareerGameResult")
                        .doc("Take one option at an open choice node; it's paid at once and the path moves on.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            String campaignId = call.string(0);
                            String nodeId = call.string(1);
                            String optionId = call.string(2);
                            return CareerApi.store(() -> CareerCampaigns.get().choose(user, campaignId, nodeId, optionId));
                        }),

                // ---- puzzles
                RpcMethod.named("careerPuzzles")
                        .returns("CareerPuzzle[]")
                        .doc("The puzzle book in order: each puzzle's position, stars and attempts. A puzzle opens when the one "
                                + "before it is solved; the first three are always open.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerPuzzles.get().puzzles(user));
                        }),
                RpcMethod.named("careerPuzzlePlay")
                        .params(of("puzzleId", STRING))
                        .returns("CareerMatch")
                        .doc("Try a puzzle: the game starts from its position on your turn, and you lose at the end of the turn. "
                                + "Stars go by attempts: three on the first try, two within three.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            String puzzleId = call.string(0);
                            CareerSetup.Puzzle puzzle = CareerApi.store(() -> CareerPuzzles.get().attempt(user, puzzleId));
                            DeckCardLists placeholder = CareerPuzzles.placeholderDeckFor("Plains");
                            DeckCardLists opponentPlaceholder = CareerPuzzles.placeholderDeckFor("Swamp");
                            String tableId = CareerApi.startMatch(ctx, call, user, "Puzzle", "Constructed - Freeform", placeholder,
                                    PlayerType.COMPUTER_MAD, 4, opponentPlaceholder,
                                    id -> CareerService.get().register(id, user, "puzzle", puzzle.id, CareerSetup.ofPuzzle(puzzle)));
                            return match(tableId, "puzzle:" + puzzle.id);
                        }),

                // ---- the weekly challenge
                RpcMethod.named("careerChallenge")
                        .returns("CareerChallenge")
                        .doc("This week's challenge (the same fixed deck and opponent for everyone), your best result and the board.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerChallenges.get().view(user));
                        }),
                RpcMethod.named("careerChallengePlay")
                        .returns("CareerMatch")
                        .doc("Play this week's challenge. A win counts on the board by fewest turns, then most life left.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            CareerChallenges.Plan plan = plan(() -> CareerChallenges.get().attempt(user));
                            String tableId = CareerApi.startMatch(ctx, call, user, plan.challenge.opponent.name, "Constructed - Freeform",
                                    plan.playerDeck, CareerApi.aiType(plan.challenge.opponent.aiType), plan.challenge.opponent.skill, plan.opponentDeck,
                                    id -> CareerService.get().register(id, user, "challenge", plan.week, plan.setup));
                            return match(tableId, "challenge:" + plan.week, plan.opponentDeck);
                        }),

                // ---- the gauntlet
                RpcMethod.named("careerGauntlet")
                        .returns("CareerGauntletState")
                        .doc("The gauntlet: the entry price, rewards by wins, the run in progress (with its deck, bosses and any "
                                + "pick waiting) and recent runs.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerGauntlet.get().state(user));
                        }),
                RpcMethod.named("careerGauntletStart")
                        .params(optionalObject("options", "StartOptions"))
                        .returns("CareerRun")
                        .doc("Pay the entry and start a run: it offers starting packs to pick from. Options (all optional): mode "
                                + "(map, bossrush or endless), variant (jumpstart, pauper, singleton, cube or draft), ascension (0 up to the "
                                + "highest open), seed (a shared seed like K7Q2-9F3M), cubeId (one of getDraftCubes) or cubeCards (card "
                                + "names of a deck used as the cube), setCode (the set to draft).")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            CareerGauntlet.StartOptions options = call.has(0)
                                    ? call.object(0, CareerGauntlet.StartOptions.class) : new CareerGauntlet.StartOptions();
                            return CareerApi.store(() -> CareerGauntlet.get().start(user, options));
                        }),
                RpcMethod.named("careerGauntletMove")
                        .params(of("nodeId", STRING))
                        .returns("CareerRun")
                        .doc("Move to a node of the run's map that the current node links to: a fight waits to be played, a shop, "
                                + "rest or event makes its offer.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            String nodeId = call.string(0);
                            return CareerApi.store(() -> CareerGauntlet.get().move(user, nodeId));
                        }),
                RpcMethod.named("careerGauntletPick")
                        .params(of("optionIds", ARRAY))
                        .returns("CareerRun")
                        .doc("Take the waiting offer: as many starting packs as it asks for, or one bundle after a win.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            List<String> ids = new ArrayList<>();
                            call.jsonArray(0).forEach(element -> ids.add(element.getAsString()));
                            return CareerApi.store(() -> CareerGauntlet.get().pick(user, ids));
                        }),
                RpcMethod.named("careerGauntletPlay")
                        .returns("CareerMatch")
                        .doc("Play the run's next boss with the run's deck.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            CareerGauntlet.Plan plan = plan(() -> CareerGauntlet.get().plan(user));
                            String tableId = CareerApi.startMatch(ctx, call, user, plan.boss.name, "Limited", plan.playerDeck,
                                    CareerApi.aiType(plan.boss.aiType), plan.skill, plan.opponentDeck,
                                    id -> CareerService.get().register(id, user, "gauntlet", plan.runId, plan.setup));
                            return match(tableId, "gauntlet:" + plan.runId, plan.opponentDeck);
                        }),
                RpcMethod.named("careerGauntletAbandon")
                        .returns("CareerRun")
                        .doc("Give up the run in progress. It pays nothing.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            return CareerApi.store(() -> CareerGauntlet.get().abandon(user));
                        }),

                // ---- solo sealed and draft
                RpcMethod.named("careerLimited")
                        .returns("CareerLimitedState")
                        .doc("Solo sealed and draft: prices, rewards by wins, the run in progress and recent runs.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerLimited.get().state(user));
                        }),
                RpcMethod.named("careerLimitedStart")
                        .params(of("kind", STRING), of("setCode", STRING))
                        .returns("CareerLimitedRun")
                        .doc("Pay the entry and start a sealed or draft run of a set the shop sells. Sealed opens six packs at once; "
                                + "a draft opens three per seat with seven bots. Every card opened joins the collection.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            String kind = call.string(0);
                            String setCode = call.string(1);
                            return CareerApi.store(() -> CareerLimited.get().start(user, kind, setCode));
                        }),
                RpcMethod.named("careerLimitedPick")
                        .params(of("index", INT))
                        .returns("CareerLimitedRun")
                        .doc("Draft: take the card at this position of the pack in front of you.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            int index = call.integer(0);
                            return CareerApi.store(() -> CareerLimited.get().pick(user, index));
                        }),
                RpcMethod.named("careerLimitedCoach")
                        .returns("CareerLimitedCoach")
                        .doc("The pool coach: every colour pair of the run's pool ranked by its ratings, removal, bombs and curve, "
                                + "each with its reasons and a 40-card build to start from.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            return CareerApi.store(() -> CareerLimited.get().coach(user));
                        }),
                RpcMethod.named("careerLimitedDeck")
                        .params(object("deck", "DeckCardLists"))
                        .returns("CareerLimitedRun")
                        .doc("Submit the run's deck: at least 40 cards from the pool, plus any basic lands.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            DeckCardLists deck = call.object(0, DeckCardLists.class);
                            return CareerApi.store(() -> CareerLimited.get().submitDeck(user, deck));
                        }),
                RpcMethod.named("careerLimitedPlay")
                        .returns("CareerMatch")
                        .doc("Play the run's next game. The run ends at three wins or two losses.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            CareerLimited.Plan plan = plan(() -> CareerLimited.get().plan(user));
                            String tableId = CareerApi.startMatch(ctx, call, user, plan.opponentName, "Limited", plan.playerDeck,
                                    PlayerType.COMPUTER_MAD, CareerLimited.OPPONENT_SKILL, plan.opponentDeck,
                                    id -> CareerService.get().register(id, user, "limited", plan.runId, null));
                            return match(tableId, "limited:" + plan.runId, plan.opponentDeck);
                        }),
                RpcMethod.named("careerLimitedAbandon")
                        .returns("CareerLimitedRun")
                        .doc("Give up the run in progress. Cards already opened stay in the collection.")
                        .handler(call -> {
                            String user = profiled(ctx, call);
                            return CareerApi.store(() -> CareerLimited.get().abandon(user));
                        })
        );
    }

    /** the signed-in account, which must have a Career */
    private static String profiled(ApiContext ctx, RpcCall call) throws RpcException {
        String user = CareerApi.career(ctx, call);
        if (CareerApi.store(() -> CareerStore.get().profile(user)) == null) {
            throw RpcException.invalidParams("Open a Career first.");
        }
        return user;
    }

    interface PlanCall<T> {
        T run() throws Exception;
    }

    private static <T> T plan(PlanCall<T> work) throws RpcException {
        try {
            return work.run();
        } catch (CareerService.CareerException e) {
            throw RpcException.invalidParams(e.getMessage());
        } catch (IOException e) {
            throw new RpcException(RpcException.SERVER_ERROR, "A deck for this game can't be read", e);
        } catch (RpcException e) {
            throw e;
        } catch (Exception e) {
            throw new RpcException(RpcException.SERVER_ERROR, "Career progress can't be read right now", e);
        }
    }

    private static CareerApi.CareerMatch match(String tableId, String opponentId) {
        return match(tableId, opponentId, null);
    }

    private static CareerApi.CareerMatch match(String tableId, String opponentId, DeckCardLists opponentDeck) {
        CareerApi.CareerMatch match = new CareerApi.CareerMatch();
        match.tableId = tableId;
        match.opponentId = opponentId;
        match.warm = CareerApi.printings(opponentDeck);
        return match;
    }
}
