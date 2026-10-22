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
import mage.server.solo.GhostDecks;
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
                        .doc("A game against the computer: start this turn again at a new table (the same cards in the same "
                                + "places, the same draws to come), and leave this game. Free play: as often as you like. Career: once a "
                                + "game outside puzzles and trials, and the game then pays half (a take-back).")
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
                            SoloViews.SoloTable table;
                            try {
                                table = start(ctx, call, user, fork);
                            } catch (RpcException | RuntimeException e) {
                                if (fork.info.career != null) {
                                    // the game carries on where it was, still playing for its Career pay
                                    CareerService.get().adopt(tableId, fork.info.career);
                                }
                                throw e;
                            }
                            try {
                                ctx.server.matchQuit(gameId, call.sessionId());
                            } catch (Exception e) {
                                logger.warn("Solo: the forked game couldn't be left", e);
                            }
                            return table;
                        }),

                RpcMethod.named("soloTurns")
                        .params(gameId())
                        .returns("SoloTurn[]")
                        .doc("The turns of one of your games against the computer that can be played again (a what-if line), "
                                + "oldest first. Every turn of your latest games is kept, running or over.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            List<SoloViews.SoloTurn> turns = new ArrayList<>();
                            for (SoloStore.TurnRow row : store(() -> SoloGames.get().turns(call.uuid(0), user))) {
                                turns.add(SoloViews.SoloTurn.of(row));
                            }
                            return turns;
                        }),
                RpcMethod.named("soloWhatIf")
                        .params(gameId(), of("turn", INT), of("sessionId", STRING))
                        .session(2)
                        .returns("SoloTable")
                        .doc("Play one of your games against the computer again from the start of one of its turns, at a new "
                                + "table: a what-if line, always free play. A running game you're in is kept as a saved game first, "
                                + "untouched, and left; a Career game can be tried again this way once it's over.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            UUID gameId = call.uuid(0);
                            UUID liveTable = SoloGames.get().tableOfGame(gameId);
                            if (liveTable != null && SoloGames.get().isCareerGame(gameId)) {
                                throw RpcException.invalidParams("Finish the Career game first: then play any turn of it again from its replay.");
                            }
                            SoloGames.Resume line;
                            try {
                                line = store(() -> SoloGames.get().whatIf(gameId, call.integer(1), user));
                            } catch (IllegalArgumentException e) {
                                throw RpcException.invalidParams(e.getMessage());
                            }
                            if (liveTable != null) {
                                // the game it branches from stays as it was: kept, to resume later
                                String problem = SoloGames.get().keep(liveTable, user);
                                if (problem != null) {
                                    throw RpcException.invalidParams(problem);
                                }
                            }
                            SoloViews.SoloTable table = start(ctx, call, user, line);
                            if (liveTable != null) {
                                try {
                                    ctx.server.matchQuit(gameId, call.sessionId());
                                } catch (Exception e) {
                                    logger.warn("Solo: the game a what-if branched from couldn't be left", e);
                                }
                            }
                            return table;
                        }),
                RpcMethod.named("soloAssists")
                        .params(gameId(), of("sessionId", STRING))
                        .session(1)
                        .returns("Assists")
                        .doc("What you may still ask for in this game against the computer: hints (the AI's advice) and taking "
                                + "back the turn. Unlimited in free play; in Career three hints and one take-back a game, outside puzzles "
                                + "and trials, and any of them halves the game's pay.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            UUID tableId = SoloGames.get().tableOfGame(call.uuid(0));
                            return SoloGames.get().assists(tableId, user);
                        }),

                // ---- advice
                RpcMethod.named("gameAdvise")
                        .params(gameId(), of("sessionId", STRING), optional("skill", INT))
                        .session(1)
                        .returns("Advice")
                        .doc("What the AI would do in your place right now, without doing it: what to play, how to attack or block. "
                                + "Only in a game against the computer; ask while the game waits for you. In Career it's a hint: three a game, "
                                + "none in puzzles and trials, and the game then pays half.")
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

                // ---- puzzles from boards (S030)
                RpcMethod.named("gamePuzzleExport")
                        .params(gameId(), of("sessionId", STRING), optional("name", STRING), optional("text", STRING), optional("objective", STRING))
                        .session(1)
                        .returns("PuzzleExport")
                        .doc("The board of a game against the computer as a puzzle (set it up with the practice tools first): both "
                                + "sides' life, hands, permanents (tapped or not), graveyards and the top of each library. Objective "
                                + "win (this turn, the default) or survive (their turn). What a card name can't bring back, like tokens "
                                + "and counters, is listed in left. Ask while the game waits for you.")
                        .handler(call -> {
                            GameController controller = ctx.managers.gameManager().getGameController().get(call.uuid(0));
                            if (controller == null) {
                                throw RpcException.invalidParams("That game isn't running.");
                            }
                            mage.server.Session session = ctx.managers.sessionManager().getSession(call.sessionId())
                                    .orElseThrow(() -> RpcException.notAuthorized("Sign in first"));
                            StringBuilder problem = new StringBuilder();
                            java.util.concurrent.CompletableFuture<mage.server.solo.PuzzleKit.PuzzleExport> export = controller.exportPuzzle(
                                    session.getUserId(), call.has(2) ? call.string(2) : null, call.has(3) ? call.string(3) : null,
                                    call.has(4) ? call.string(4) : null, problem);
                            if (export == null) {
                                throw RpcException.invalidParams(problem.toString());
                            }
                            try {
                                return export.get(10, java.util.concurrent.TimeUnit.SECONDS);
                            } catch (java.util.concurrent.TimeoutException e) {
                                throw RpcException.invalidParams("Save the puzzle when the game is waiting for you to act.");
                            } catch (InterruptedException e) {
                                Thread.currentThread().interrupt();
                                throw RpcException.invalidParams("Interrupted.");
                            } catch (java.util.concurrent.ExecutionException e) {
                                throw RpcException.invalidParams("The board couldn't be read: " + e.getCause().getMessage());
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
                        }),

                // ---- free play against the meta, ghosts and adaptive skill
                RpcMethod.named("metaDeck")
                        .params(of("file", STRING))
                        .returns("DeckCardLists")
                        .doc("A deck from the meta library (see metaLibrary), for the computer to pilot in free play.")
                        .handler(call -> {
                            CareerApi.user(ctx, call);
                            mage.server.career.MetaLibrary.MetaDeck meta = mage.server.career.MetaLibrary.get().deck(call.string(0));
                            if (meta == null) {
                                throw RpcException.invalidParams("The meta library has no deck " + call.string(0) + ".");
                            }
                            try {
                                mage.cards.decks.DeckCardLists deck = mage.server.career.CareerCampaigns.get().deck(meta.file);
                                if (deck == null) {
                                    throw new IOException("no deck");
                                }
                                return deck;
                            } catch (IOException e) {
                                throw new RpcException(RpcException.SERVER_ERROR, "That deck couldn't be read.", e);
                            }
                        }),
                RpcMethod.named("ghostShare")
                        .params(optional("name", STRING), object("deck", "DeckCardLists"))
                        .returns("GhostDeck")
                        .doc("Share a deck as a ghost under a short code: anyone with the code can play against it, piloted by "
                                + "the computer, and each result reaches your inbox.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            String name = call.has(0) ? call.string(0) : null;
                            mage.cards.decks.DeckCardLists deck = call.object(1, mage.cards.decks.DeckCardLists.class);
                            return store(() -> GhostDecks.get().share(user, name, deck, System.currentTimeMillis()));
                        }),
                RpcMethod.named("ghostMine")
                        .returns("GhostDeck[]")
                        .doc("The decks you share as ghosts, newest first, with their records.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            return store(() -> GhostDecks.get().mine(user));
                        }),
                RpcMethod.named("ghostUnshare")
                        .params(of("code", STRING))
                        .returns("boolean")
                        .doc("Stop sharing a ghost deck.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            return store(() -> GhostDecks.get().unshare(user, call.string(0)));
                        }),
                RpcMethod.named("ghostDeck")
                        .params(of("code", STRING))
                        .returns("GhostOpponent")
                        .doc("The ghost deck shared under a code, to seat the computer with.")
                        .handler(call -> {
                            CareerApi.user(ctx, call);
                            return store(() -> GhostDecks.get().opponent(call.string(0)));
                        }),
                RpcMethod.named("soloTableMark")
                        .params(of("tableId", mage.server.websocket.rpc.RpcParam.Type.UUID), optional("ghostCode", STRING),
                                optional("adaptiveFormat", STRING))
                        .returns("boolean")
                        .doc("Before starting a table of yours against the computer: play it against a ghost (its result goes to "
                                + "the deck's owner), or with adaptive skill (its result moves your hidden rating for the format).")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            UUID tableId = call.uuid(0);
                            mage.game.Table table = ctx.managers.tableManager().getTable(tableId);
                            if (table == null || !user.equals(table.getControllerName())) {
                                throw RpcException.notAuthorized("That isn't your table.");
                            }
                            if (call.has(1)) {
                                store(() -> {
                                    GhostDecks.get().mark(tableId, user, call.string(1));
                                    return null;
                                });
                            }
                            if (call.has(2)) {
                                mage.server.career.AdaptiveSkill.mark(tableId, user, adaptiveFormat(call.string(2)));
                            }
                            return true;
                        }),
                RpcMethod.named("soloAdaptive")
                        .params(of("format", STRING))
                        .returns("AdaptiveView")
                        .doc("Your adaptive skill for a format (\"career\", \"constructed\", \"commander\"...): the skill the next "
                                + "game plays at and how it's going, as a sentence.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            String format = adaptiveFormat(call.string(0));
                            return store(() -> mage.server.career.AdaptiveSkill.view(user, format));
                        }),
                RpcMethod.named("inbox")
                        .returns("InboxNote[]")
                        .doc("Your latest notes, newest first: how your ghost decks did.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            return store(() -> GhostDecks.get().inbox(user, 50));
                        }),
                RpcMethod.named("inboxSeen")
                        .returns("boolean")
                        .doc("Mark every note in your inbox seen.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            return store(() -> {
                                SoloStore.get().markInboxSeen(user);
                                return true;
                            });
                        })
        );
    }

    private static String adaptiveFormat(String format) throws RpcException {
        String value = format == null ? "" : format.trim().toLowerCase(java.util.Locale.ENGLISH);
        if (!value.matches("[a-z0-9-]{1,24}")) {
            throw RpcException.invalidParams("A format is a short name like constructed.");
        }
        return value;
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
