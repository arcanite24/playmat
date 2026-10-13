package mage.server.websocket.api;

import mage.game.advice.Advice;
import mage.game.match.MatchOptions;
import mage.players.PlayerType;
import mage.server.admin.RuntimeSettings;
import mage.server.admin.StaffRole;
import mage.server.career.CareerService;
import mage.server.game.GameController;
import mage.server.solo.AdviceService;
import mage.server.solo.BatchRunner;
import mage.server.solo.ContentPacks;
import mage.server.solo.Leaderboards;
import mage.server.solo.SoloGames;
import mage.server.solo.SoloStore;
import mage.server.solo.SoloViews;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;
import mage.view.TableView;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Base64;
import java.util.List;
import java.util.UUID;

import static mage.server.websocket.rpc.RpcParam.Type.ARRAY;
import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.object;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: single-player foundations. Saved solo games (resume, delete, save and leave), "try this turn
 * again", advice from the AI, AI-against-AI batches, leaderboards, and the staff side of batches and content packs.
 */
final class SoloApi {

    private static final Logger logger = Logger.getLogger(SoloApi.class);

    private SoloApi() {
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                // ---- saved games
                RpcMethod.named("soloSaves")
                        .returns("SoloSave[]")
                        .doc("Your saved games against the computer, newest first. Every solo game saves itself at the start of each "
                                + "turn; one still running says so (rejoin it from your open games).")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            List<SoloViews.SoloSave> saves = new ArrayList<>();
                            for (SoloStore.SavedRow row : store(() -> SoloGames.get().list(user))) {
                                saves.add(SoloViewsAccess.save(row, SoloGames.get().isRunning(row.id)));
                            }
                            return saves;
                        }),
                RpcMethod.named("soloResume")
                        .params(of("saveId", STRING))
                        .returns("SoloTable")
                        .doc("Carry on a saved game from the start of the turn it was saved in, at a new table with the same "
                                + "opponents and decks. A Career game still pays when it ends.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            SoloGames.Resume resume = store(() -> SoloGames.get().load(call.string(0), user));
                            return start(ctx, call, user, resume);
                        }),
                RpcMethod.named("soloDelete")
                        .params(of("saveId", STRING))
                        .returns("boolean")
                        .doc("Throw a saved game away.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            return store(() -> SoloGames.get().delete(call.string(0), user));
                        }),
                RpcMethod.named("soloSaveAndLeave")
                        .params(gameId(), of("sessionId", STRING))
                        .session(1)
                        .returns("boolean")
                        .doc("Free play against the computer: keep the game as saved at the start of this turn, and leave it. "
                                + "Resume it later from your saved games.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            UUID gameId = call.uuid(0);
                            UUID tableId = SoloGames.get().tableOfGame(gameId);
                            String problem = SoloGames.get().keep(tableId, user);
                            if (problem != null) {
                                throw RpcException.invalidParams(problem);
                            }
                            ctx.server.matchQuit(gameId, call.sessionId());
                            return true;
                        }),
                RpcMethod.named("soloFork")
                        .params(gameId(), of("sessionId", STRING))
                        .session(1)
                        .returns("SoloTable")
                        .doc("Free play against the computer: start this turn again at a new table (the same cards in the same "
                                + "places, the same draws to come), and leave this game.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            UUID gameId = call.uuid(0);
                            UUID tableId = SoloGames.get().tableOfGame(gameId);
                            SoloGames.Resume fork;
                            try {
                                fork = SoloGames.get().forkOf(tableId, user);
                            } catch (IllegalArgumentException e) {
                                throw RpcException.invalidParams(e.getMessage());
                            }
                            SoloViews.SoloTable table = start(ctx, call, user, fork);
                            try {
                                ctx.server.matchQuit(gameId, call.sessionId());
                            } catch (Exception e) {
                                logger.warn("Solo: the forked game couldn't be left", e);
                            }
                            return table;
                        }),

                // ---- advice
                RpcMethod.named("gameAdvise")
                        .params(gameId(), of("sessionId", STRING), optional("skill", INT))
                        .session(1)
                        .returns("Advice")
                        .doc("What the AI would do in your place right now, without doing it: what to play, how to attack or block. "
                                + "Only in a game against the computer, outside Career; ask while the game waits for you.")
                        .handler(call -> {
                            GameController controller = ctx.managers.gameManager().getGameController().get(call.uuid(0));
                            if (controller == null) {
                                throw RpcException.invalidParams("That game isn't running.");
                            }
                            mage.server.Session session = ctx.managers.sessionManager().getSession(call.sessionId())
                                    .orElseThrow(() -> RpcException.notAuthorized("Sign in first"));
                            try {
                                return AdviceService.get().advise(controller, session.getUserId(),
                                        call.has(2) ? call.integer(2) : AdviceService.DEFAULT_SKILL);
                            } catch (IllegalArgumentException e) {
                                throw RpcException.invalidParams(e.getMessage());
                            }
                        }),

                // ---- batches
                RpcMethod.named("soloBatchSubmit")
                        .params(object("spec", "JobSpec"), optional("kind", STRING))
                        .returns("BatchJob")
                        .doc("Queue AI-against-AI games between two decks (1 to 500, each seeded: game n plays seed + n). "
                                + "The server plays a few at a time; ask for the job to see the score.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            BatchRunner.JobSpec spec = call.object(0, BatchRunner.JobSpec.class);
                            if (RuntimeSettings.get().batchConcurrency() <= 0) {
                                throw RpcException.invalidParams("The server isn't playing batches right now.");
                            }
                            try {
                                return SoloViews.BatchJob.of(store(() -> BatchRunner.get().submit(user, call.optString(1, null), spec)), false);
                            } catch (IllegalArgumentException e) {
                                throw RpcException.invalidParams(e.getMessage());
                            }
                        }),
                RpcMethod.named("soloBatchJobs")
                        .returns("BatchJob[]")
                        .doc("Your batches, newest first, with their scores so far.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            List<SoloViews.BatchJob> jobs = new ArrayList<>();
                            for (SoloStore.JobRow row : store(() -> SoloStore.get().jobs(user, 50))) {
                                jobs.add(SoloViews.BatchJob.of(row, false));
                            }
                            return jobs;
                        }),
                RpcMethod.named("soloBatchJob")
                        .params(of("jobId", STRING))
                        .returns("BatchJob")
                        .doc("One of your batches with every game's seed, winner and length.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            SoloStore.JobRow row = store(() -> SoloStore.get().job(call.string(0)));
                            if (row == null || !row.user.equals(user)) {
                                throw RpcException.invalidParams("There's no such batch.");
                            }
                            return SoloViews.BatchJob.of(row, true);
                        }),
                RpcMethod.named("soloBatchCancel")
                        .params(of("jobId", STRING))
                        .returns("boolean")
                        .doc("Stop one of your batches; games already played keep their score.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            return store(() -> BatchRunner.get().cancel(call.string(0), user));
                        }),

                // ---- leaderboards
                RpcMethod.named("leaderboard")
                        .params(of("mode", STRING), of("board", STRING), optional("scope", STRING), optional("names", ARRAY))
                        .returns("Leaderboard")
                        .doc("A mode's leaderboard (weekly-challenge: board is the week, like 2026-W42; puzzle: board is the puzzle "
                                + "id), best first, with your line. Scope server (everyone) or friends (the names given, and you).")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            List<String> names = new ArrayList<>();
                            if (call.has(3)) {
                                call.jsonArray(3).forEach(element -> names.add(element.getAsString()));
                            }
                            try {
                                return store(() -> Leaderboards.get().board(call.string(0), call.string(1), user,
                                        call.optString(2, Leaderboards.SCOPE_SERVER), names, 50));
                            } catch (IllegalArgumentException e) {
                                throw RpcException.invalidParams(e.getMessage());
                            }
                        }),

                // ---- staff
                RpcMethod.named("adminBatchJobs")
                        .returns("BatchStatus")
                        .doc("Staff (admin): the batch runner's settings, games running now and the latest jobs of every player.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.ADMIN);
                            SoloViews.BatchStatus status = new SoloViews.BatchStatus();
                            status.concurrency = RuntimeSettings.get().batchConcurrency();
                            status.gamesPerDay = RuntimeSettings.get().batchGamesPerDay();
                            status.gamesInFlight = BatchRunner.get().gamesInFlight();
                            for (SoloStore.JobRow row : store(() -> SoloStore.get().jobs(null, 100))) {
                                status.jobs.add(SoloViews.BatchJob.of(row, false));
                            }
                            return status;
                        }),
                RpcMethod.named("adminBatchCancel")
                        .params(of("jobId", STRING))
                        .returns("boolean")
                        .doc("Staff (admin): stop anyone's batch.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            boolean cancelled = store(() -> BatchRunner.get().cancel(call.string(0), null));
                            if (cancelled) {
                                Staff.audit(actor, "batch.cancel", null, call.string(0), null);
                            }
                            return cancelled;
                        }),
                RpcMethod.named("adminContentPacks")
                        .returns("ContentPackView[]")
                        .doc("Staff (admin): the content packs on the server, each loaded or with the problems that kept it out.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.ADMIN);
                            return ContentPacks.get().list();
                        }),
                RpcMethod.named("adminContentPackUpload")
                        .params(of("zipBase64", STRING), optional("reason", STRING))
                        .returns("ContentPackView")
                        .doc("Staff (owner): add or replace a content pack from a zip (pack.json and decks/*.dck, up to 4 MB). "
                                + "It's checked as it loads; one with problems stays out and says why.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.OWNER);
                            byte[] zip;
                            try {
                                zip = Base64.getDecoder().decode(call.string(0));
                            } catch (IllegalArgumentException e) {
                                throw RpcException.invalidParams("The upload isn't base64.");
                            }
                            try {
                                ContentPacks.ContentPackView view = ContentPacks.get().install(zip);
                                Staff.audit(actor, "contentPack.upload", null, view.id + (view.loaded ? "" : " (not loaded)"),
                                        call.optString(1, null));
                                return view;
                            } catch (IllegalArgumentException e) {
                                throw RpcException.invalidParams(e.getMessage());
                            } catch (IOException e) {
                                throw new RpcException(RpcException.SERVER_ERROR, "The pack couldn't be written: " + e.getMessage(), e);
                            }
                        }),
                RpcMethod.named("adminContentPackRemove")
                        .params(of("packId", STRING), optional("reason", STRING))
                        .returns("ContentPackView[]")
                        .doc("Staff (owner): remove a content pack. Progress players made in it stays, but its content goes.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.OWNER);
                            try {
                                if (!ContentPacks.get().remove(call.string(0))) {
                                    throw RpcException.invalidParams("There's no pack " + call.string(0) + ".");
                                }
                            } catch (IOException e) {
                                throw new RpcException(RpcException.SERVER_ERROR, "The pack couldn't be removed: " + e.getMessage(), e);
                            }
                            Staff.audit(actor, "contentPack.remove", null, call.string(0), call.optString(1, null));
                            return ContentPacks.get().list();
                        }),
                RpcMethod.named("adminContentPackReload")
                        .returns("ContentPackView[]")
                        .doc("Staff (admin): read the content pack folder again (after copying a pack in by hand).")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            List<ContentPacks.ContentPackView> views = ContentPacks.get().reload();
                            Staff.audit(actor, "contentPack.reload", null, null, null);
                            return views;
                        })
        );
    }

    private static mage.server.websocket.rpc.RpcParam gameId() {
        return of("gameId", mage.server.websocket.rpc.RpcParam.Type.UUID);
    }

    /** sets up a table for a resumed or forked game and starts it */
    static SoloViews.SoloTable start(ApiContext ctx, RpcCall call, String user, SoloGames.Resume resume) throws RpcException {
        SoloGames.SavedTable info = resume.info;
        if (!user.equals(info.user)) {
            throw RpcException.notAuthorized("That isn't your game.");
        }
        String sessionId = call.sessionId();
        UUID roomId = ctx.managers.gamesRoomManager().getMainRoomId();
        MatchOptions options = info.options;
        options.setPassword("");
        UUID tableId = null;
        try {
            TableView table = ctx.server.roomCreateTable(sessionId, roomId, options);
            tableId = table.getTableId();
            SoloGames.get().expectResume(tableId, resume);
            SoloGames.SavedSeat human = null;
            for (SoloGames.SavedSeat seat : info.seats) {
                if (seat.human) {
                    human = seat;
                }
            }
            if (human == null || !ctx.server.roomJoinTable(sessionId, roomId, tableId, user, PlayerType.HUMAN, 1, human.deck, "")) {
                throw RpcException.invalidParams("Your deck wasn't accepted for the table.");
            }
            for (SoloGames.SavedSeat seat : info.seats) {
                if (seat.human) {
                    continue;
                }
                if (!ctx.server.roomJoinTable(sessionId, roomId, tableId, seat.name, seat.type, Math.max(1, seat.skill), seat.deck, "")) {
                    throw new RpcException(RpcException.SERVER_ERROR, seat.name + " couldn't take a seat.");
                }
            }
            if (!ctx.server.matchStart(sessionId, roomId, tableId)) {
                throw new RpcException(RpcException.SERVER_ERROR, "The game couldn't start.");
            }
        } catch (RpcException e) {
            abandon(ctx, sessionId, roomId, tableId);
            throw e;
        } catch (Exception e) {
            abandon(ctx, sessionId, roomId, tableId);
            throw new RpcException(RpcException.SERVER_ERROR, "The game couldn't be set up again: " + e.getMessage(), e);
        }
        SoloViews.SoloTable result = new SoloViews.SoloTable();
        result.tableId = tableId.toString();
        return result;
    }

    private static void abandon(ApiContext ctx, String sessionId, UUID roomId, UUID tableId) {
        if (tableId == null) {
            return;
        }
        SoloGames.get().cancelResume(tableId);
        CareerService.get().forget(tableId);
        try {
            ctx.server.tableRemove(sessionId, roomId, tableId);
        } catch (Exception e) {
            logger.warn("Solo: an unfinished table couldn't be removed", e);
        }
    }

    interface StoreCall<T> {
        T run() throws SQLException, RpcException;
    }

    static <T> T store(StoreCall<T> work) throws RpcException {
        try {
            return work.run();
        } catch (SQLException e) {
            logger.error("Solo store failed", e);
            throw new RpcException(RpcException.SERVER_ERROR, "The server couldn't reach its saved games right now.", e);
        } catch (IllegalArgumentException e) {
            throw RpcException.invalidParams(e.getMessage());
        } catch (IllegalStateException e) {
            throw new RpcException(RpcException.SERVER_ERROR, e.getMessage(), e);
        }
    }

    /** SoloSave.of is package-private to the solo package */
    private static final class SoloViewsAccess {
        static SoloViews.SoloSave save(SoloStore.SavedRow row, boolean running) {
            return SoloViews.SoloSave.ofRow(row, running);
        }
    }
}
