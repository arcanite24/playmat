package mage.server.decks;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

/**
 * A deck shared with the server (#137), as the shared list shows it: whose it is and a summary read from the deck's
 * own JSON. Only deckSharedGet carries the data.
 */
public class SharedDeck {

    /** the owner's account name */
    public String owner;
    public String deckId;
    public String name;
    /** the owner's last change, client time */
    public long updatedAt;
    public long sharedAt;
    /** the deck type ("Constructed - Standard"), when the deck has one */
    public String format;
    /** WUBRG letters of the deck's colors, in that order */
    public String colors;
    public int cardCount;
    public int sideboardCount;
    public String coverSetCode;
    public String coverCardNumber;
    public String coverName;
    /** shared by the account asking */
    public boolean mine;
    /** the deck's JSON; only in deckSharedGet */
    public String data;

    public SharedDeck() {
    }

    SharedDeck(StoredDeckShare share, StoredDeck deck, String viewer, boolean withData) {
        this.owner = share.ownerName == null || share.ownerName.isEmpty() ? share.owner : share.ownerName;
        this.deckId = deck.deckId;
        this.name = deck.name;
        this.updatedAt = deck.updatedAt;
        this.sharedAt = share.sharedAt;
        this.mine = viewer != null && share.owner.equals(viewer);
        this.data = withData ? deck.data : null;
        summarize(deck.data);
    }

    /** Format, colors, counts and cover from the deck's JSON; a deck that isn't readable keeps only its name. */
    private void summarize(String json) {
        if (json == null || json.isEmpty()) {
            return;
        }
        try {
            JsonElement parsed = JsonParser.parseString(json);
            if (!parsed.isJsonObject()) {
                return;
            }
            JsonObject deck = parsed.getAsJsonObject();
            format = text(deck, "format");
            cardCount = count(deck.get("cards"));
            sideboardCount = count(deck.get("sideboard"));
            JsonElement colorFlags = deck.get("colors");
            if (colorFlags != null && colorFlags.isJsonObject()) {
                StringBuilder letters = new StringBuilder();
                String[][] order = {{"white", "W"}, {"blue", "U"}, {"black", "B"}, {"red", "R"}, {"green", "G"}};
                for (String[] color : order) {
                    JsonElement on = colorFlags.getAsJsonObject().get(color[0]);
                    if (on != null && on.isJsonPrimitive() && on.getAsJsonPrimitive().isBoolean() && on.getAsBoolean()) {
                        letters.append(color[1]);
                    }
                }
                colors = letters.toString();
            }
            JsonElement cover = deck.get("coverCard");
            if (cover != null && cover.isJsonObject()) {
                coverSetCode = text(cover.getAsJsonObject(), "setCode");
                coverCardNumber = text(cover.getAsJsonObject(), "cardNumber");
                coverName = text(cover.getAsJsonObject(), "name");
            }
        } catch (RuntimeException e) {
            // not the client's JSON: the list shows the name alone
        }
    }

    private static String text(JsonObject object, String field) {
        JsonElement value = object.get(field);
        return value != null && value.isJsonPrimitive() && value.getAsJsonPrimitive().isString() ? value.getAsString() : null;
    }

    private static int count(JsonElement list) {
        if (list == null || !list.isJsonArray()) {
            return 0;
        }
        int total = 0;
        JsonArray entries = list.getAsJsonArray();
        for (JsonElement entry : entries) {
            if (entry.isJsonObject()) {
                JsonElement amount = entry.getAsJsonObject().get("amount");
                if (amount != null && amount.isJsonPrimitive() && amount.getAsJsonPrimitive().isNumber()) {
                    total += Math.max(0, amount.getAsInt());
                }
            }
        }
        return total;
    }
}
