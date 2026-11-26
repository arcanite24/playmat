package mage.server.websocket.rpc;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonDeserializer;
import com.google.gson.JsonElement;
import com.google.gson.JsonNull;
import com.google.gson.JsonObject;
import com.google.gson.JsonPrimitive;
import com.google.gson.JsonSerializer;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.view.CardView;
import mage.view.SimpleCardView;

import java.util.Date;
import java.util.UUID;

/**
 * Web client bridge: the single JSON mapping used for requests, responses and server callbacks.
 * <p>
 * Java view classes are serialized field by field (Gson reflection), with these conventions:
 * UUIDs are strings, dates are epoch milliseconds and {@link SimpleCardView} carries its card name.
 */
public final class JsonCodec {

    public static final Gson GSON = new GsonBuilder()
            .registerTypeAdapter(UUID.class, (JsonSerializer<UUID>) (src, type, context) ->
                    src == null ? JsonNull.INSTANCE : new JsonPrimitive(src.toString()))
            .registerTypeAdapter(UUID.class, (JsonDeserializer<UUID>) (json, type, context) ->
                    json == null || json.isJsonNull() ? null : UUID.fromString(json.getAsString()))
            .registerTypeHierarchyAdapter(Date.class, (JsonSerializer<Date>) (src, type, context) ->
                    src == null ? JsonNull.INSTANCE : new JsonPrimitive(src.getTime()))
            .registerTypeHierarchyAdapter(Date.class, (JsonDeserializer<Date>) (json, type, context) ->
                    json == null || json.isJsonNull() ? null : new Date(json.getAsLong()))
            // a whole card view where a simple one is declared (the hand of a player whose turn you control) stays whole
            .registerTypeAdapter(SimpleCardView.class, (JsonSerializer<SimpleCardView>) (src, type, context) ->
                    src instanceof CardView ? context.serialize(src, CardView.class) : serializeSimpleCard(src))
            .create();

    // simple views are serialized for every draft pick and deck, so names come from a cache, not one query per card
    private static final CardNameCache CARD_NAMES = new CardNameCache(
            (setCode, cardNumber) -> {
                CardInfo cardInfo = CardRepository.instance.findCard(setCode, cardNumber);
                return cardInfo == null ? null : cardInfo.getName();
            },
            () -> CardRepository.instance.getContentChanges(), // lambda: keep the database closed until a lookup
            CardNameCache.DEFAULT_MAX_ENTRIES
    );

    private JsonCodec() {
    }

    private static JsonElement serializeSimpleCard(SimpleCardView src) {
        JsonObject json = new JsonObject();
        json.addProperty("id", src.getId() == null ? null : src.getId().toString());
        json.addProperty("expansionSetCode", src.getExpansionSetCode());
        json.addProperty("setCode", src.getExpansionSetCode());
        json.addProperty("cardNumber", src.getCardNumber());
        json.addProperty("usesVariousArt", src.getUsesVariousArt());
        json.addProperty("gameObject", src.isGameObject());
        json.addProperty("isChoosable", src.isChoosable());
        json.addProperty("isSelected", src.isSelected());

        // simple views only know set/number, but a client needs the name to render text-only states
        String setCode = src.getExpansionSetCode();
        String cardNumber = src.getCardNumber();
        if (setCode != null && cardNumber != null) {
            String name = CARD_NAMES.findName(setCode, cardNumber);
            if (name != null) {
                json.addProperty("name", name);
                json.addProperty("displayName", name);
            }
        }
        return json;
    }
}
