package mage.server.support;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParseException;
import com.google.gson.JsonParser;

import java.io.UnsupportedEncodingException;
import java.math.BigDecimal;
import java.math.RoundingMode;
import java.net.URLDecoder;
import java.util.Locale;

/**
 * One Ko-fi webhook delivery: a form post whose single field {@code data} holds the payment as JSON (Ko-fi, Settings,
 * API, Webhooks). Ko-fi signs nothing; its {@code verification_token} is the shared secret that proves the sender.
 */
public final class KofiPayload {

    public final String verificationToken;
    public final String transactionId;
    /** Donation, Subscription, Commission or Shop Order */
    public final String type;
    public final boolean isPublic;
    public final String fromName;
    public final String message;
    public final long amountCents;
    public final String currency;
    public final String email;
    public final boolean subscription;
    public final boolean firstSubscription;
    public final String tierName;
    /** the JSON as Ko-fi sent it, for staff, kept 90 days */
    public final String json;

    private KofiPayload(JsonObject data, String json) {
        this.json = json;
        verificationToken = string(data, "verification_token");
        transactionId = string(data, "kofi_transaction_id");
        type = string(data, "type");
        isPublic = bool(data, "is_public");
        fromName = string(data, "from_name");
        message = string(data, "message");
        amountCents = cents(string(data, "amount"));
        String code = string(data, "currency");
        currency = code == null ? "USD" : code.trim().toUpperCase(Locale.ROOT);
        email = string(data, "email");
        subscription = bool(data, "is_subscription_payment");
        firstSubscription = bool(data, "is_first_subscription_payment");
        tierName = string(data, "tier_name");
    }

    /**
     * Reads the form body Ko-fi posts.
     *
     * @throws IllegalArgumentException when it isn't a Ko-fi payment
     */
    public static KofiPayload parse(String formBody) {
        String json = null;
        for (String pair : (formBody == null ? "" : formBody).split("&")) {
            int eq = pair.indexOf('=');
            if (eq > 0 && "data".equals(decode(pair.substring(0, eq)))) {
                json = decode(pair.substring(eq + 1));
                break;
            }
        }
        if (json == null || json.trim().isEmpty()) {
            throw new IllegalArgumentException("no data field");
        }
        JsonElement parsed;
        try {
            parsed = JsonParser.parseString(json);
        } catch (JsonParseException e) {
            throw new IllegalArgumentException("data is not JSON");
        }
        if (!parsed.isJsonObject()) {
            throw new IllegalArgumentException("data is not a JSON object");
        }
        KofiPayload payload = new KofiPayload(parsed.getAsJsonObject(), json);
        if (payload.transactionId == null || payload.transactionId.length() > 100) {
            throw new IllegalArgumentException("no kofi_transaction_id");
        }
        if (payload.amountCents < 0) {
            throw new IllegalArgumentException("amount is not a number");
        }
        return payload;
    }

    /** The ledger's kind for Ko-fi's type. */
    public String kind() {
        String text = type == null ? "" : type.trim().toLowerCase(Locale.ROOT);
        if (subscription || text.equals("subscription")) {
            return "membership";
        }
        if (text.equals("shop order")) {
            return "shop";
        }
        if (text.equals("commission")) {
            return "commission";
        }
        return "donation";
    }

    private static String string(JsonObject data, String name) {
        JsonElement value = data.get(name);
        if (value == null || value.isJsonNull() || !value.isJsonPrimitive()) {
            return null;
        }
        String text = value.getAsString();
        return text.trim().isEmpty() ? null : text;
    }

    private static boolean bool(JsonObject data, String name) {
        JsonElement value = data.get(name);
        if (value == null || value.isJsonNull() || !value.isJsonPrimitive()) {
            return false;
        }
        return value.getAsJsonPrimitive().isBoolean() ? value.getAsBoolean() : "true".equalsIgnoreCase(value.getAsString());
    }

    /** "3.00" as 300; -1 when it isn't an amount */
    static long cents(String amount) {
        if (amount == null) {
            return -1;
        }
        try {
            BigDecimal value = new BigDecimal(amount.trim().replace(",", ""));
            if (value.signum() < 0 || value.compareTo(new BigDecimal(1_000_000)) > 0) {
                return -1;
            }
            return value.setScale(2, RoundingMode.HALF_UP).movePointRight(2).longValueExact();
        } catch (NumberFormatException | ArithmeticException e) {
            return -1;
        }
    }

    private static String decode(String text) {
        try {
            return URLDecoder.decode(text, "UTF-8");
        } catch (UnsupportedEncodingException | IllegalArgumentException e) {
            return text;
        }
    }
}
