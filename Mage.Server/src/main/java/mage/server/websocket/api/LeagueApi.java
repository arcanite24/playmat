package mage.server.websocket.api;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import mage.cards.decks.DeckCardLists;
import mage.server.career.CareerLeague;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.object;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * The Career League (#433): start a format's League with two starters, open its packs, and keep League decks built
 * from the Career collection. The games themselves are ranked games in the League queue (ladderState / ladderOpponent
 * with queue "league", and leaguePlay).
 */
public final class LeagueApi {

    private LeagueApi() {
    }

    public static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("leagueState")
                        .returns("LeagueState")
                        .doc("Your Leagues: per format whether it's started, its pack tokens and decks, and the starters to pick "
                                + "from before it is; plus the wildcards and coins to build with.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            return CareerApi.store(() -> CareerLeague.get().state(user));
                        }),

                RpcMethod.named("leagueStart")
                        .params(of("format", STRING), object("starters", "string[]"))
                        .returns("LeagueStart")
                        .doc("Start the format's League with two of its starters (their ids): their cards join the collection and "
                                + "each becomes a League deck, with the format's packs and, with your first League, wildcards.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String format = call.string(0);
                            List<String> starters = new ArrayList<>();
                            JsonArray array = call.jsonArray(1);
                            for (JsonElement element : array) {
                                if (!element.isJsonPrimitive()) {
                                    throw RpcException.invalidParams("starters is a list of starter ids");
                                }
                                starters.add(element.getAsString());
                            }
                            return CareerApi.store(() -> CareerLeague.get().start(user, format, starters));
                        }),

                RpcMethod.named("leagueSets")
                        .params(of("format", STRING))
                        .returns("LeagueSet[]")
                        .doc("The sets the format's League opens packs of, newest first; Pioneer's also has the mixed pack "
                                + "(setCode \"mixed\").")
                        .handler(call -> {
                            CareerApi.career(ctx, call);
                            String format = call.string(0);
                            return CareerApi.store(() -> CareerLeague.get().sets(format));
                        }),

                RpcMethod.named("leagueOpen")
                        .params(of("format", STRING), of("setCode", STRING))
                        .returns("LeaguePack")
                        .doc("Open a pack of the format's League: the format's pack token first, then a Career pack token, then "
                                + "coins. The cards join the collection.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String format = call.string(0);
                            String setCode = call.string(1);
                            return CareerApi.store(() -> CareerLeague.get().openPack(user, format, setCode));
                        }),

                RpcMethod.named("leagueDecks")
                        .params(optional("format", STRING))
                        .returns("LeagueDeck[]")
                        .doc("Your League decks (of one format, or all), newest first, each with the cards you're missing and "
                                + "why the format refuses it, if it does.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String format = call.optString(0, null);
                            return CareerApi.store(() -> CareerLeague.get().decks(user, format));
                        }),

                RpcMethod.named("leagueSaveDeck")
                        .params(optional("id", STRING), of("name", STRING), of("format", STRING), object("deck", "DeckCardLists"))
                        .returns("LeagueDeck")
                        .doc("Save a League deck: a new one without an id. It may be unfinished; what keeps it from being played "
                                + "comes back with it.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String id = call.optString(0, null);
                            String name = call.string(1);
                            String format = call.string(2);
                            DeckCardLists deck = call.object(3, DeckCardLists.class);
                            return CareerApi.store(() -> CareerLeague.get().saveDeck(user, id, name, format, deck));
                        }),

                RpcMethod.named("leagueDeleteDeck")
                        .params(of("id", STRING))
                        .returns("boolean")
                        .doc("Delete one of your League decks. The cards stay in the collection.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            String id = call.string(0);
                            return CareerApi.store(() -> CareerLeague.get().deleteDeck(user, id));
                        })
        );
    }
}
