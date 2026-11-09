package mage.server.websocket.api;

import com.google.gson.JsonElement;
import com.google.gson.JsonPrimitive;
import mage.constants.ManaType;
import mage.constants.PlayerAction;
import mage.server.game.GameController;
import mage.server.game.PracticeTools;
import mage.server.replay.ReplayStore;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.UUID;

import static mage.server.websocket.rpc.RpcParam.Type.ANY;
import static mage.server.websocket.rpc.RpcParam.Type.BOOLEAN;
import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.OBJECT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: playing, watching and replaying games.
 */
final class GameApi {

    private GameApi() {
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("gameJoin")
                        .session(1)
                        .params(of("gameId", mage.server.websocket.rpc.RpcParam.Type.UUID), of("sessionId", STRING))
                        .doc("Join a started game as a player (after START_GAME). Also used to resync after a reconnect.")
                        .handler(call -> {
                            ctx.server.gameJoin(call.uuid(0), call.string(1));
                            return true;
                        }),

                RpcMethod.named("hostGameEnded")
                        .publicAccess()
                        .params(gameId(), of("token", STRING), optional("winnerId", STRING), of("turns", INT),
                                of("reason", STRING), optional("timeLeft", OBJECT))
                        .doc("The gateway only: a game the Rust host played ended (docs/rust/GameHost.md). The token is "
                                + "the one the game's HOST_GAME carried; timeLeft is each player's priority time left in "
                                + "seconds, by player id, for a timed match's next game. Returns false for a game that "
                                + "isn't hosted.")
                        .handler(call -> mage.server.RustHost.ended(call.uuid(0), call.string(1), call.optString(2, ""),
                                call.integer(3), call.string(4), call.has(5) ? call.jsonObject(5) : null)),

                RpcMethod.named("gameResync")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING))
                        .doc("Send this connection the game's open question again, when a reply or the question seems lost. "
                                + "Returns false when nothing is waiting for the player's answer.")
                        .handler(call -> call.getConnection().resendPrompt(call.uuid(0))),

                RpcMethod.named("gameStateResync")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING))
                        .doc("With state patches on (setCapabilities): the client could not apply a patch, so the server sends "
                                + "the game's last state complete (as GAME_UPDATE or REPLAY_UPDATE), then its open question. "
                                + "Returns false when it has none; the next state is sent complete anyway.")
                        .handler(call -> call.getConnection().resendGameState(call.uuid(0))),

                RpcMethod.named("matchQuit")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING))
                        .doc("Concede the whole match.")
                        .handler(call -> {
                            ctx.server.matchQuit(call.uuid(0), call.string(1));
                            return true;
                        }),

                RpcMethod.named("gameWatchStart")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING))
                        .doc("Start spectating a game.")
                        .handler(call -> ctx.server.gameWatchStart(call.uuid(0), call.string(1))),

                RpcMethod.named("replayList")
                        .returns("ReplayInfo[]")
                        .doc("Recorded games available for replay, newest first. Games are recorded as a spectator sees them "
                                + "(xmage.replays, on by default) and kept for xmage.replays.retentionDays (30).")
                        .handler(call -> {
                            ctx.sessions.userName(call.sessionId())
                                    .orElseThrow(() -> RpcException.notAuthorized("Log in to see replays"));
                            return ReplayStore.get().list();
                        }),

                RpcMethod.named("replayPage")
                        .params(gameId(), of("start", INT), of("count", INT))
                        .returns("ReplayPage | null")
                        .doc("Events of a recorded game, in order: board snapshots ({t, view, myHand?}) and game log lines ({t, log}). "
                                + "At most " + ReplayStore.MAX_PAGE + " per page; myHand is the caller's own hand in games they played. "
                                + "Null when there is no such replay.")
                        .handler(call -> {
                            String userName = ctx.sessions.userName(call.sessionId())
                                    .orElseThrow(() -> RpcException.notAuthorized("Log in to watch replays"));
                            try {
                                return ReplayStore.get().page(call.uuid(0), call.integer(1), call.integer(2), userName);
                            } catch (java.io.IOException e) {
                                throw new RpcException(RpcException.SERVER_ERROR, "The replay can't be read: " + e.getMessage(), e);
                            }
                        }),

                RpcMethod.named("gameWatchers")
                        .params(gameId())
                        .returns("string[]")
                        .doc("Names of the users spectating a running game (empty when the game is over or unknown).")
                        .handler(call -> {
                            GameController controller = ctx.managers.gameManager().getGameController().get(call.uuid(0));
                            return controller == null ? Collections.emptyList() : controller.getWatcherNames();
                        }),

                RpcMethod.named("gameWatchStop")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING))
                        .doc("Stop spectating a game.")
                        .handler(call -> {
                            call.getConnection().forgetGameState(call.uuid(0));
                            ctx.server.gameWatchStop(call.uuid(0), call.string(1));
                            return true;
                        }),

                RpcMethod.named("sendPlayerUUID")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING), optional("data", mage.server.websocket.rpc.RpcParam.Type.UUID))
                        .doc("Answer a prompt with an object or player id (targets, choices, attackers, blockers).")
                        .handler(call -> {
                            answered(call, 0);
                            ctx.server.sendPlayerUUID(call.uuid(0), call.string(1), call.optUuid(2));
                            return true;
                        }),

                RpcMethod.named("sendPlayerString")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING), of("data", STRING))
                        .doc("Answer a prompt with text (choices, \"special\" mana payment).")
                        .handler(call -> {
                            answered(call, 0);
                            ctx.server.sendPlayerString(call.uuid(0), call.string(1), call.string(2));
                            return true;
                        }),

                RpcMethod.named("sendPlayerBoolean")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING), of("data", BOOLEAN))
                        .doc("Answer a yes/no prompt, pass priority (false) or cancel mana payment (any value).")
                        .handler(call -> {
                            answered(call, 0);
                            ctx.server.sendPlayerBoolean(call.uuid(0), call.string(1), call.bool(2));
                            return true;
                        }),

                RpcMethod.named("sendPlayerInteger")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING), of("data", INT))
                        .doc("Answer an amount prompt.")
                        .handler(call -> {
                            answered(call, 0);
                            ctx.server.sendPlayerInteger(call.uuid(0), call.string(1), call.integer(2));
                            return true;
                        }),

                RpcMethod.named("sendPlayerManaType")
                        .session(2)
                        .params(gameId(), of("playerId", mage.server.websocket.rpc.RpcParam.Type.UUID), of("sessionId", STRING),
                                of("manaType", STRING).typed("ManaType"))
                        .doc("Pay from the mana pool with one mana type.")
                        .handler(call -> {
                            answered(call, 0);
                            ctx.server.sendPlayerManaType(call.uuid(0), call.uuid(1), call.string(2), manaType(call, 3));
                            return true;
                        }),

                RpcMethod.named("sendPlayerAction")
                        .session(2)
                        .params(of("action", STRING).typed("PlayerAction"), gameId(), of("sessionId", STRING), optional("data", ANY))
                        .doc("Send a player action: pass modes (F-keys), concede, undo, rollback, auto-answer and trigger-order settings.")
                        .handler(call -> {
                            PlayerAction action = playerAction(call, 0);
                            if (action == PlayerAction.UNDO || action == PlayerAction.CONCEDE || action.name().startsWith("PASS_PRIORITY_")) {
                                // these answer (or replace) the open question
                                answered(call, 1);
                            }
                            ctx.server.sendPlayerAction(action, call.uuid(1), call.string(2),
                                    call.has(3) ? actionData(call.raw(3)) : null);
                            return true;
                        }),

                RpcMethod.named("cheatShow")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING), of("playerId", mage.server.websocket.rpc.RpcParam.Type.UUID))
                        .doc("Test mode only: reveal a player's library.")
                        .handler(call -> {
                            ctx.server.cheatShow(call.uuid(0), call.string(1), call.uuid(2));
                            return true;
                        }),

                RpcMethod.named("gamePractice")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING), of("tool", STRING), optional("cardName", STRING), optional("amount", INT))
                        .doc("Practice tools in a game against the AI only: tool HAND or BATTLEFIELD (cardName, amount cards), DRAW (amount), UNTAP_ALL, LIFE (amount). Runs the next time you are asked something; announced in the game log.")
                        .handler(call -> {
                            GameController controller = ctx.managers.gameManager().getGameController().get(call.uuid(0));
                            if (controller == null) {
                                throw new RpcException(RpcException.INVALID_PARAMS, "That game isn't running.");
                            }
                            mage.server.Session session = ctx.managers.sessionManager().getSession(call.sessionId())
                                    .orElseThrow(() -> RpcException.notAuthorized("Sign in first"));
                            PracticeTools.Tool tool = PracticeTools.parse(call.string(2));
                            if (tool == null) {
                                throw new RpcException(RpcException.INVALID_PARAMS, "Unknown practice tool: " + call.string(2));
                            }
                            String problem = controller.practice(session.getUserId(), tool,
                                    call.has(3) ? call.string(3) : null, call.has(4) ? call.integer(4) : 1);
                            if (problem != null) {
                                throw new RpcException(RpcException.INVALID_PARAMS, problem);
                            }
                            return true;
                        }),

                replay(ctx, "replayInit", "Open a saved game for replay."),
                replay(ctx, "replayStart", "Start the replay."),
                replay(ctx, "replayStop", "Stop the replay."),
                replay(ctx, "replayNext", "Step forward."),
                replay(ctx, "replayPrevious", "Step back."),

                RpcMethod.named("replaySkipForward")
                        .session(1)
                        .params(gameId(), of("sessionId", STRING), of("moves", INT))
                        .doc("Skip several steps forward.")
                        .handler(call -> {
                            ctx.server.replaySkipForward(call.uuid(0), call.string(1), call.integer(2));
                            return true;
                        })
        );
    }

    /**
     * Forgets the open question before the answer is passed on, so the next question can't be cleared by mistake.
     */
    private static void answered(RpcCall call, int gameIdIndex) throws RpcException {
        call.getConnection().promptAnswered(call.uuid(gameIdIndex));
    }

    private static mage.server.websocket.rpc.RpcParam gameId() {
        return of("gameId", mage.server.websocket.rpc.RpcParam.Type.UUID);
    }

    private static RpcMethod replay(ApiContext ctx, String name, String description) {
        return RpcMethod.named(name)
                .session(1)
                .params(gameId(), of("sessionId", STRING))
                .doc(description)
                .handler(call -> {
                    UUID gameId = call.uuid(0);
                    String sessionId = call.string(1);
                    switch (name) {
                        case "replayInit":
                            ctx.server.replayInit(gameId, sessionId);
                            break;
                        case "replayStart":
                            ctx.server.replayStart(gameId, sessionId);
                            break;
                        case "replayStop":
                            call.getConnection().forgetGameState(gameId);
                            ctx.server.replayStop(gameId, sessionId);
                            break;
                        case "replayNext":
                            ctx.server.replayNext(gameId, sessionId);
                            break;
                        case "replayPrevious":
                            ctx.server.replayPrevious(gameId, sessionId);
                            break;
                        default:
                            throw new IllegalStateException("Unknown replay method " + name);
                    }
                    return true;
                });
    }

    private static ManaType manaType(RpcCall call, int index) throws RpcException {
        String value = call.string(index);
        try {
            return ManaType.valueOf(value);
        } catch (IllegalArgumentException e) {
            throw RpcException.invalidParams("Unknown mana type: " + value);
        }
    }

    private static PlayerAction playerAction(RpcCall call, int index) throws RpcException {
        String value = call.string(index);
        try {
            return PlayerAction.valueOf(value);
        } catch (IllegalArgumentException e) {
            throw RpcException.invalidParams("Unknown player action: " + value);
        }
    }

    /**
     * Player actions carry a UUID, a number, a boolean or a string, depending on the action.
     */
    private static Object actionData(JsonElement data) throws RpcException {
        if (!data.isJsonPrimitive()) {
            throw RpcException.invalidParams("Player action data must be a string, number or boolean");
        }
        JsonPrimitive primitive = data.getAsJsonPrimitive();
        if (primitive.isBoolean()) {
            return primitive.getAsBoolean();
        }
        if (primitive.isNumber()) {
            return primitive.getAsInt();
        }
        String text = primitive.getAsString();
        try {
            return UUID.fromString(text);
        } catch (IllegalArgumentException e) {
            return text;
        }
    }
}
