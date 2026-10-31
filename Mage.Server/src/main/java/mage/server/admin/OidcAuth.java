package mage.server.admin;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.nimbusds.jwt.JWTClaimsSet;
import org.apache.log4j.Logger;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URI;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.security.SecureRandom;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Collections;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.function.LongSupplier;

/**
 * Sign in with the server's identity provider (OpenID Connect), addressed through its discovery document:
 * off unless {@code xmage.oidc.discovery}, {@code xmage.oidc.clientId}, {@code xmage.oidc.clientSecret}
 * and {@code xmage.publicUrl} are all set (the provider sends the player back to {@code <publicUrl>/auth/oidc}).
 * The server exchanges the one-use code, verifies the ID token against the provider's published keys
 * (see OidcVerifier) and reads its claims. With {@code xmage.oidc.only} the provider is the only sign-in
 * method. The client secret is a secret and is never logged.
 */
public final class OidcAuth {

    private static final Logger logger = Logger.getLogger(OidcAuth.class);

    public static final String DISCOVERY_PROP = "xmage.oidc.discovery";
    public static final String CLIENT_ID_PROP = "xmage.oidc.clientId";
    public static final String CLIENT_SECRET_PROP = "xmage.oidc.clientSecret";
    public static final String LABEL_PROP = "xmage.oidc.label";
    public static final String ONLY_PROP = "xmage.oidc.only";
    public static final String PUBLIC_URL_PROP = "xmage.publicUrl";

    public static final String CALLBACK_PATH = "/auth/oidc";
    static final long STATE_MILLIS = TimeUnit.MINUTES.toMillis(10);
    static final long DISCOVERY_TTL = TimeUnit.HOURS.toMillis(24);
    static final long DISCOVERY_RETRY = TimeUnit.MINUTES.toMillis(5);
    static final int MAX_PENDING = 500;

    /** One HTTP exchange with the identity provider: the response body, or an IOException. */
    @FunctionalInterface
    public interface Http {
        String request(String method, String url, String token, String formBody) throws IOException;
    }

    /**
     * The provider's discovery document, with relative endpoints resolved against the document's URL.
     */
    public static final class Discovery {
        public final String issuer;
        public final String authorizationEndpoint;
        public final String tokenEndpoint;
        public final String jwksUri;
        /** null when the provider doesn't offer a userinfo endpoint */
        public final String userinfoEndpoint;
        /** the ID token algorithms the provider promises (id_token_signing_alg_values_supported) */
        public final List<String> algs;

        public Discovery(String issuer, String authorizationEndpoint, String tokenEndpoint, String jwksUri,
                         String userinfoEndpoint, List<String> algs) {
            this.issuer = issuer;
            this.authorizationEndpoint = authorizationEndpoint;
            this.tokenEndpoint = tokenEndpoint;
            this.jwksUri = jwksUri;
            this.userinfoEndpoint = userinfoEndpoint;
            this.algs = algs;
        }
    }

    /** Who signed in: the provider's stable user id (sub), the email and whether the provider verified it. */
    public static final class Identity {
        public final String sub;
        public final String email;
        public final boolean emailVerified;
        public final String displayName;

        public Identity(String sub, String email, boolean emailVerified, String displayName) {
            this.sub = sub;
            this.email = email;
            this.emailVerified = emailVerified;
            this.displayName = displayName;
        }
    }

    /** state → when it stops being accepted, and the nonce its ID token must carry */
    static final class OidcPending {
        final long until;
        final String nonce;

        OidcPending(long until, String nonce) {
            this.until = until;
            this.nonce = nonce;
        }
    }

    private static volatile OidcAuth instance;
    private static final SecureRandom random = new SecureRandom();

    private final String discoveryUrl;
    private final String clientId;
    private final String clientSecret;
    private final String label;
    private final String publicUrl;
    private final boolean only;
    private final Http http;
    private final LongSupplier clock;
    private final OidcVerifier verifier;
    private volatile Discovery discovery;
    private volatile long discoveryAt;
    private volatile String discoveryProblem;
    /** state → when it stops being accepted */
    private final Map<String, OidcPending> pending = new ConcurrentHashMap<>();
    /** state → who it signed in, so naming the new account doesn't need the provider's one-use code again */
    private final Map<String, Identity> identities = new ConcurrentHashMap<>();

    public OidcAuth(String discoveryUrl, String clientId, String clientSecret, String label, String publicUrl,
                    boolean oidcOnly, Http http, LongSupplier clock) {
        this.discoveryUrl = trim(discoveryUrl);
        this.clientId = trim(clientId);
        this.clientSecret = trim(clientSecret);
        String name = trim(label);
        this.label = name.isEmpty() ? "OIDC" : name;
        this.publicUrl = trim(publicUrl).replaceAll("/+$", "");
        this.only = oidcOnly;
        this.http = http;
        this.clock = clock;
        this.verifier = new OidcVerifier(http, clock, this.clientId);
    }

    public static OidcAuth get() {
        OidcAuth current = instance;
        if (current == null) {
            synchronized (OidcAuth.class) {
                current = instance;
                if (current == null) {
                    current = new OidcAuth(System.getProperty(DISCOVERY_PROP, ""), System.getProperty(CLIENT_ID_PROP, ""),
                            System.getProperty(CLIENT_SECRET_PROP, ""), System.getProperty(LABEL_PROP, ""),
                            System.getProperty(PUBLIC_URL_PROP, ""), Boolean.parseBoolean(System.getProperty(ONLY_PROP, "false")),
                            OidcAuth::httpRequest, System::currentTimeMillis);
                    current.startDiscovery();
                    instance = current;
                }
            }
        }
        return current;
    }

    /** For tests. */
    public static void use(OidcAuth auth) {
        instance = auth;
    }

    public boolean isEnabled() {
        return !discoveryUrl.isEmpty() && !clientId.isEmpty() && !clientSecret.isEmpty() && !publicUrl.isEmpty();
    }

    /** Alias for isEnabled(). */
    public boolean enabled() {
        return isEnabled();
    }

    public String label() {
        return label;
    }

    /** the provider's issuer from the discovery document, or empty until it is known */
    public String issuer() {
        Discovery current = discovery;
        return current == null ? "" : current.issuer;
    }

    /** true when this server signs in with the provider only (xmage.oidc.only) */
    public boolean isOidcOnly() {
        return only;
    }

    /**
     * The one player-readable refusal every password path gives while this server signs in with the provider only:
     * it says nothing about the account, so it leaks nothing.
     */
    public String refusal() {
        return "This server signs in with " + label + " only.";
    }

    /** why the last discovery fetch failed, or null */
    public String discoveryProblem() {
        return discoveryProblem;
    }

    // --- sign-in ----------------------------------------------------------------------------------

    /**
     * Where to send the player: the provider's authorization page, with a one-use state (and a nonce its
     * ID token must carry) that comes back to us.
     */
    public String start() {
        if (!isEnabled()) {
            throw new IllegalStateException("Sign in with " + label + " isn't set up on this server");
        }
        Discovery current = discovery;
        if (current == null) {
            throw new IllegalStateException(label + " can't be reached right now. Try again in a minute.");
        }
        long now = clock.getAsLong();
        prune(now);
        if (pending.size() >= MAX_PENDING) {
            throw new IllegalStateException("Too many " + label + " sign-ins at once. Try again in a few minutes.");
        }
        String state = randomToken();
        String nonce = randomToken();
        pending.put(state, new OidcPending(now + STATE_MILLIS, nonce));
        return current.authorizationEndpoint + "?client_id=" + encode(clientId)
                + "&redirect_uri=" + encode(publicUrl + CALLBACK_PATH)
                + "&scope=" + encode("openid profile email").replace("+", "%20")
                + "&response_type=code"
                + "&state=" + encode(state)
                + "&nonce=" + encode(nonce);
    }

    /**
     * Who came back from the provider with this state and code. The code is used once; asking again with the
     * same state (to pick an account name) returns the same person without it.
     */
    public Identity identify(String state, String code) throws IOException {
        long now = clock.getAsLong();
        prune(now);
        Identity known = state == null ? null : identities.get(state);
        if (known != null) {
            return known;
        }
        OidcPending spot = state == null ? null : pending.remove(state);
        if (spot == null || spot.until < now) {
            throw new IllegalArgumentException("That sign-in with " + label + " has expired. Start again.");
        }
        if (code == null || code.trim().isEmpty()) {
            throw new IllegalArgumentException(label + " didn't send a sign-in code. Start again.");
        }
        Discovery current = discovery;
        if (current == null) {
            throw new IllegalStateException(label + " can't be reached right now. Try again in a minute.");
        }
        String form = "grant_type=authorization_code&code=" + encode(code.trim())
                + "&redirect_uri=" + encode(publicUrl + CALLBACK_PATH)
                + "&client_id=" + encode(clientId)
                + "&client_secret=" + encode(clientSecret);
        JsonObject exchange = JsonParser.parseString(http.request("POST", current.tokenEndpoint, null, form)).getAsJsonObject();
        if (exchange.has("error") || !exchange.has("id_token") || exchange.get("id_token").getAsString().trim().isEmpty()) {
            throw new IllegalArgumentException(label + " refused the sign-in. Start again.");
        }
        String accessToken = exchange.has("access_token") ? exchange.get("access_token").getAsString() : null;
        JWTClaimsSet claims = verifier.verify(exchange.get("id_token").getAsString(), spot.nonce, current);
        String sub = claims.getSubject();
        if (sub == null || sub.trim().isEmpty()) {
            throw new IllegalArgumentException(label + " refused the sign-in. Start again.");
        }
        String email = stringClaim(claims, "email");
        boolean emailVerified = Boolean.TRUE.equals(claims.getClaim("email_verified"));
        if (email == null && current.userinfoEndpoint != null && accessToken != null && !accessToken.trim().isEmpty()) {
            try {
                JsonObject profile = JsonParser.parseString(http.request("GET", current.userinfoEndpoint, accessToken, null)).getAsJsonObject();
                email = text(profile, "email");
                emailVerified = profile.has("email_verified") && profile.get("email_verified").getAsBoolean();
            } catch (IOException | RuntimeException e) {
                logger.info(label + " profile not readable for " + sub);
            }
        }
        Identity identity = new Identity(sub, email, emailVerified, displayOf(claims, email));
        identities.put(state, identity);
        pending.put("identity:" + state, new OidcPending(now + STATE_MILLIS, spot.nonce)); // how long the identity is kept
        return identity;
    }

    /** The state's sign-in is finished: forget who it was. */
    public void done(String state) {
        if (state != null) {
            identities.remove(state);
            pending.remove("identity:" + state);
        }
    }

    private void prune(long now) {
        for (Iterator<Map.Entry<String, OidcPending>> it = pending.entrySet().iterator(); it.hasNext(); ) {
            Map.Entry<String, OidcPending> entry = it.next();
            if (entry.getValue().until < now) {
                it.remove();
                if (entry.getKey().startsWith("identity:")) {
                    identities.remove(entry.getKey().substring("identity:".length()));
                }
            }
        }
    }

    /**
     * A name the account could have here: provider display names allow almost anything, account names don't.
     */
    public static String suggestName(String displayName) {
        String name = displayName == null ? "" : displayName.replaceAll("[^A-Za-z0-9_]", "_");
        if (name.length() > 14) {
            name = name.substring(0, 14);
        }
        while (name.length() < 3) {
            name = name + "_";
        }
        return name;
    }

    // --- the discovery document -------------------------------------------------------------------

    /**
     * Fetches the provider's discovery document. The background thread runs this at boot, then every 24 hours
     * (every 5 minutes while it fails); tests run it directly.
     */
    public void fetchDiscovery() {
        if (!isEnabled()) {
            return;
        }
        long now = clock.getAsLong();
        if (discovery != null && now - discoveryAt < DISCOVERY_TTL) {
            return; // the document is still fresh
        }
        try {
            String json = http.request("GET", discoveryUrl, null, null);
            discovery = parseDiscovery(json);
            discoveryAt = clock.getAsLong();
            discoveryProblem = null;
            logger.info(label + " discovered: " + discovery.issuer);
        } catch (IOException | RuntimeException e) {
            discoveryProblem = e.getClass().getSimpleName() + (e.getMessage() == null ? "" : ": " + e.getMessage());
            logger.warn(label + " can't be reached: " + discoveryProblem);
        }
    }

    private void startDiscovery() {
        if (!isEnabled()) {
            return;
        }
        ScheduledExecutorService executor = Executors.newSingleThreadScheduledExecutor(runnable -> {
            Thread thread = new Thread(runnable, "XMAGE oidc discovery");
            thread.setDaemon(true);
            return thread;
        });
        scheduleRefresh(executor, 0);
    }

    private void scheduleRefresh(ScheduledExecutorService executor, long delayMillis) {
        executor.schedule(() -> {
            fetchDiscovery();
            long delay = discovery == null ? DISCOVERY_RETRY : DISCOVERY_TTL;
            scheduleRefresh(executor, delay);
        }, delayMillis, TimeUnit.MILLISECONDS);
    }

    private Discovery parseDiscovery(String json) throws IOException {
        JsonObject doc = JsonParser.parseString(json).getAsJsonObject();
        String issuer = text(doc, "issuer");
        String authorize = resolve(text(doc, "authorization_endpoint"));
        String token = resolve(text(doc, "token_endpoint"));
        String jwks = resolve(text(doc, "jwks_uri"));
        String userinfo = resolve(text(doc, "userinfo_endpoint"));
        if (isBlank(issuer) || isBlank(authorize) || isBlank(token) || isBlank(jwks)) {
            throw new IOException("the discovery document is missing its endpoints");
        }
        List<String> algs = new ArrayList<>();
        if (doc.has("id_token_signing_alg_values_supported") && doc.get("id_token_signing_alg_values_supported").isJsonArray()) {
            for (JsonElement element : doc.getAsJsonArray("id_token_signing_alg_values_supported")) {
                algs.add(element.getAsString());
            }
        }
        return new Discovery(issuer, authorize, token, jwks, isBlank(userinfo) ? null : userinfo, Collections.unmodifiableList(algs));
    }

    /** A relative endpoint in the document is relative to the document's URL. */
    private String resolve(String endpoint) {
        if (endpoint == null || !endpoint.startsWith("/")) {
            return endpoint;
        }
        try {
            return URI.create(discoveryUrl).resolve(endpoint).toString();
        } catch (IllegalArgumentException e) {
            return endpoint;
        }
    }

    private static String text(JsonObject object, String name) {
        return object.has(name) && object.get(name).isJsonPrimitive() ? object.get(name).getAsString() : null;
    }

    private static String stringClaim(JWTClaimsSet claims, String name) {
        Object value = claims.getClaim(name);
        return value instanceof String ? (String) value : null;
    }

    /** preferred_username, then name, then the email's local part, then nothing. */
    private static String displayOf(JWTClaimsSet claims, String email) {
        Object preferred = claims.getClaim("preferred_username");
        if (preferred instanceof String && !((String) preferred).trim().isEmpty()) {
            return ((String) preferred).trim();
        }
        Object name = claims.getClaim("name");
        if (name instanceof String && !((String) name).trim().isEmpty()) {
            return ((String) name).trim();
        }
        if (email != null && email.contains("@")) {
            String local = email.substring(0, email.indexOf('@'));
            return local.isEmpty() ? null : local;
        }
        return null;
    }

    // --- HTTP -------------------------------------------------------------------------------------

    static String httpRequest(String method, String url, String token, String formBody) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) new URL(url).openConnection();
        try {
            connection.setRequestMethod(method);
            connection.setConnectTimeout(5_000);
            connection.setReadTimeout(10_000);
            connection.setRequestProperty("Accept", "application/json");
            connection.setRequestProperty("User-Agent", "playmat-server");
            if (token != null) {
                connection.setRequestProperty("Authorization", "Bearer " + token);
            }
            if (formBody != null) {
                connection.setDoOutput(true);
                connection.setRequestProperty("Content-Type", "application/x-www-form-urlencoded");
                try (OutputStream out = connection.getOutputStream()) {
                    out.write(formBody.getBytes(StandardCharsets.UTF_8));
                }
            }
            int status = connection.getResponseCode();
            if (status < 200 || status >= 300) {
                throw new IOException("the identity provider answered HTTP " + status);
            }
            try (InputStream in = connection.getInputStream()) {
                ByteArrayOutputStream bytes = new ByteArrayOutputStream();
                byte[] buffer = new byte[8192];
                int read;
                while ((read = in.read(buffer)) > 0 && bytes.size() < 2_000_000) {
                    bytes.write(buffer, 0, read);
                }
                return new String(bytes.toByteArray(), StandardCharsets.UTF_8);
            }
        } finally {
            connection.disconnect();
        }
    }

    private static String randomToken() {
        byte[] bytes = new byte[18];
        random.nextBytes(bytes);
        return Base64.getUrlEncoder().withoutPadding().encodeToString(bytes);
    }

    private static String encode(String text) {
        try {
            return URLEncoder.encode(text, "UTF-8");
        } catch (java.io.UnsupportedEncodingException e) {
            throw new IllegalStateException(e);
        }
    }

    private static String trim(String text) {
        return text == null ? "" : text.trim();
    }

    private static boolean isBlank(String text) {
        return text == null || text.trim().isEmpty();
    }
}
