package mage.server.websocket.api;

import mage.server.career.CareerCapsules;
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

                RpcMethod.named("careerProfileOf")
                        .params(of("userName", STRING))
                        .returns("CareerPublicProfile | null")
                        .doc("What anyone sees of a player's progression on their profile: level, achievements and the last "
                                + "counted games. Null when the player has none, or hid progression.")
                        .handler(call -> {
                            CareerApi.career(ctx, call);
                            String name = call.string(0);
                            return CareerApi.store(() -> {
                                mage.server.career.CareerProfile profile = CareerStore.get().profile(name);
                                return profile == null || !profile.showProgress ? null : CareerProgress.get().publicProfile(name);
                            });
                        }),

                RpcMethod.named("careerQuestTracker")
                        .params(of("gameId", mage.server.websocket.rpc.RpcParam.Type.UUID))
                        .returns("CareerLiveQuest[]")
                        .doc("The player's unfinished quests with what the running game has added so far, for the match menu. "
                                + "Nothing is saved until the game ends. Empty when the game doesn't count.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            java.util.UUID gameId = call.uuid(0);
                            mage.server.game.GameController controller = ctx.managers.gameManager().getGameController().get(gameId);
                            java.util.UUID userId = ctx.managers.sessionManager().getSession(call.sessionId())
                                    .map(mage.server.Session::getUserId).orElse(null);
                            java.util.UUID playerId = controller == null ? null : controller.playerIdOf(userId);
                            mage.server.career.CareerTally tally = playerId == null ? null : mage.server.career.CareerTally.of(gameId, playerId);
                            java.util.Map<String, Integer> counters = tally == null || tally.taint() != null
                                    ? java.util.Collections.emptyMap() : tally.counters();
                            return CareerApi.store(() -> CareerProgress.get().liveQuests(user, counters));
                        }),

                RpcMethod.named("careerMasteries")
                        .params(optional("userName", STRING))
                        .returns("CareerMastery[]")
                        .doc("Mastery tracks (each color, multicolor, colorless and each format): games played, rank of ten and "
                                + "games to the next rank. Another player's when a name is given and they show progression.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String name = call.optString(0, "").trim();
                            String who = name.isEmpty() ? user : name;
                            return CareerApi.store(() -> {
                                mage.server.career.CareerProfile profile = CareerStore.get().profile(who);
                                if (profile == null || (!who.equalsIgnoreCase(user) && !profile.showProgress)) {
                                    return java.util.Collections.emptyList();
                                }
                                return CareerProgress.get().masteries(who);
                            });
                        }),

                RpcMethod.named("careerDaily")
                        .returns("CareerDaily")
                        .doc("Today (UTC): wins so far, the daily wins ladder's coins and which step is next, and whether the "
                                + "first win and the first game against a person have paid their bonus.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerProgress.get().daily(user));
                        }),

                RpcMethod.named("careerCosmetics")
                        .returns("CareerCatalogItem[]")
                        .doc("Every cosmetic progression gives (B293): avatars, titles, frames, sleeves, playmats, companions, "
                                + "emote sets, arrows, sparks, stamps and land styles, with names, rarity, how each looks and "
                                + "where it comes from. The supporters' thank-you set is not in it.")
                        .handler(call -> {
                            CareerApi.career(ctx, call);
                            return mage.server.career.CareerCosmetics.get().items();
                        }),

                RpcMethod.named("careerMachines")
                        .returns("CareerMachines")
                        .doc("The capsule shop (M29): every capsule machine with its sets, the account's capsule tokens and coins, "
                                + "what a token costs, the capsules pulled so far and the card styles worn.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerCapsules.get().shop(user));
                        }),

                RpcMethod.named("careerBuyTokens")
                        .params(of("count", INT))
                        .returns("CareerProfile")
                        .doc("Buy capsule tokens with coins, in a bundle of 1, 5 or 10; returns the profile after.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            int count = call.integer(0);
                            return CareerApi.store(() -> CareerCapsules.get().buyTokens(user, count));
                        }),

                RpcMethod.named("careerPull")
                        .params(of("machine", STRING), of("count", INT))
                        .returns("CareerPullResult")
                        .doc("Pull 1 to 10 capsules from a machine, a token each: every capsule is an item the account doesn't have, "
                                + "picked at random. Asking for more than the machine has left gives (and charges) what's left.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String machine = call.string(0);
                            int count = call.integer(1);
                            return CareerApi.store(() -> CareerCapsules.get().pull(user, machine, count));
                        }),

                RpcMethod.named("careerEquipStyle")
                        .params(of("card", STRING), optional("item", STRING))
                        .returns("{ [card: string]: string }")
                        .doc("Wear a full-art card style on its card, or take the card's style off (no item). Returns every style worn, "
                                + "card name to style.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String card = call.string(0);
                            String item = call.optString(1, null);
                            return CareerApi.store(() -> CareerCapsules.get().equipStyle(user, card, item == null || item.isEmpty() ? null : item));
                        }),

                RpcMethod.named("careerHall")
                        .returns("CareerHall")
                        .doc("The achievement hall: every achievement with its tiers and how many players have each, the "
                                + "points total and its rank, and the latest earned.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerProgress.get().hall(user));
                        }),

                RpcMethod.named("gameEmote")
                        .params(of("gameId", mage.server.websocket.rpc.RpcParam.Type.UUID), of("emote", STRING))
                        .returns("boolean")
                        .doc("Sends a quick emote to everyone at the table (a gameEmote callback). Only from an emote set the "
                                + "account has, or the free phrases; one every few seconds. False when it wasn't sent.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            java.util.UUID gameId = call.uuid(0);
                            String emote = call.string(1);
                            mage.server.career.CareerCosmetics.CareerCatalogItem set = mage.server.career.CareerCosmetics.get().emoteSetOf(emote);
                            if (set == null) {
                                return false;
                            }
                            boolean owned = set.free || CareerApi.store(() -> CareerStore.get().unlocksOf(user).stream()
                                    .anyMatch(unlock -> "emote".equals(unlock[0]) && set.id.equals(unlock[1])));
                            if (!owned) {
                                return false;
                            }
                            mage.server.game.GameController controller = ctx.managers.gameManager().getGameController().get(gameId);
                            java.util.UUID userId = ctx.managers.sessionManager().getSession(call.sessionId())
                                    .map(mage.server.Session::getUserId).orElse(null);
                            return controller != null && controller.emote(userId, emote, System.currentTimeMillis());
                        })
        );
    }
}
