package mage.server.websocket.api;

import mage.server.career.CareerProgress;
import mage.server.career.CareerService;
import mage.server.career.CareerStore;
import mage.server.websocket.rpc.RpcMethod;

import java.util.Arrays;
import java.util.List;

import static mage.server.websocket.rpc.RpcParam.Type.BOOLEAN;
import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: what keeps a Career going between games. Daily quests, the level track, achievements, the weekly
 * goal, set progress, and what a finished game paid. All of it is earned on the server when a game ends.
 */
final class CareerProgressApi {

    private CareerProgressApi() {
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("careerQuests")
                        .returns("CareerQuests | null")
                        .doc("The quests waiting (up to three; a new one arrives each day) and whether one can be swapped today. "
                                + "Null before the account opens a Career.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerProgress.get().quests(user));
                        }),

                RpcMethod.named("careerRerollQuest")
                        .params(of("slot", INT))
                        .returns("CareerQuests")
                        .doc("Swap the quest in a slot for a different one, once a day. The new quest starts from zero.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            int slot = call.integer(0);
                            return CareerApi.store(() -> CareerProgress.get().reroll(user, slot));
                        }),

                RpcMethod.named("careerAchievements")
                        .returns("CareerAchievement[]")
                        .doc("Every achievement, earned or not, with progress on the Career-long ones. A secret one has no name "
                                + "or text until it's earned.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerProgress.get().achievements(user));
                        }),

                RpcMethod.named("careerLevels")
                        .returns("CareerLevelTrack | null")
                        .doc("The level track: the account's level and XP, what every level gives, and the cosmetics unlocked so far.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerProgress.get().levelTrack(user));
                        }),

                RpcMethod.named("careerWeekly")
                        .returns("CareerWeekly")
                        .doc("Career wins this week (ISO weeks, UTC) and the goals that each give a free pack.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerProgress.get().weekly(CareerStore.get(), user));
                        }),

                RpcMethod.named("careerGameResult")
                        .params(optional("gameKey", STRING))
                        .returns("CareerGameResult | null")
                        .doc("What a finished game paid: the match's coins and XP, quest progress, achievements, level rewards and "
                                + "the weekly goal. The key is the Career table id (or the game id of a regular AI game); "
                                + "without one, the latest game. Null when that game paid nothing.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String gameKey = call.optString(0, null);
                            return CareerApi.store(() -> CareerService.get().gameResult(user, gameKey));
                        }),

                RpcMethod.named("careerSetProgress")
                        .returns("CareerSetProgress[]")
                        .doc("How much of each set the collection has: different cards owned (any printing) out of the cards its "
                                + "packs hold, basic lands left out. Most complete first.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerService.get().setProgress(user));
                        }),

                RpcMethod.named("careerShowProgress")
                        .params(of("show", BOOLEAN))
                        .returns("CareerProfile")
                        .doc("Whether the player sees progression: the header, Today on Home and the rewards after games. Kept "
                                + "with the account, so it holds on every device. Games still count while it's hidden.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            boolean show = call.bool(0);
                            return CareerApi.store(() -> CareerService.get().setShowProgress(user, show));
                        }),

                RpcMethod.named("careerDaily")
                        .returns("CareerDaily")
                        .doc("Today (UTC): wins so far, the daily wins ladder's coins and which step is next, and whether the "
                                + "first win and the first game against a person have paid their bonus.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerProgress.get().daily(user));
                        })
        );
    }
}
