package mage.server.admin;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.apache.log4j.Logger;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Base64;
import java.util.Collections;
import java.util.HashSet;
import java.util.Iterator;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.function.LongSupplier;

/**
 * Sign in with GitHub, and a repository's collaborators let in to a contributors server automatically.
 * <p>
 * Off unless an OAuth app is configured: {@code xmage.github.clientId}, {@code xmage.github.clientSecret} and
 * {@code xmage.publicUrl} (GitHub sends the player back to {@code <publicUrl>/auth/github}). With
 * {@code xmage.github.repo} ("owner/name") the repository's people are fetched every few hours: its collaborators when
 * {@code xmage.github.token} can read them, otherwise its public contributors. A GitHub account on that list gets in
 * whatever the allowlist says. The client secret and the token are secrets and are never logged.
 */
public final class GithubAuth {

    private static final Logger logger = Logger.getLogger(GithubAuth.class);

    public static final String CLIENT_ID_PROP = "xmage.github.clientId";
    public static final String CLIENT_SECRET_PROP = "xmage.github.clientSecret";
    public static final String REPO_PROP = "xmage.github.repo";
    public static final String TOKEN_PROP = "xmage.github.token";
    public static final String PUBLIC_URL_PROP = "xmage.publicUrl";

    public static final String CALLBACK_PATH = "/auth/github";
    static final long STATE_MILLIS = TimeUnit.MINUTES.toMillis(10);
    static final long SYNC_HOURS = 6;
    private static final int MAX_PENDING = 500;

    /** One HTTP exchange with GitHub: the response body, or an IOException. */
    @FunctionalInterface
    public interface Http {
        String request(String method, String url, String token, String formBody) throws IOException;
    }

    /** Who signed in: GitHub's numeric id (stable across renames), the login, and a verified email when there is one. */
    public static final class Identity {
        public final long id;
        public final String login;
        public final String email;

        public Identity(long id, String login, String email) {
            this.id = id;
            this.login = login;
            this.email = email;
        }
    }

    private static volatile GithubAuth instance;
    private static final SecureRandom random = new SecureRandom();

    private final String clientId;
    private final String clientSecret;
    private final String publicUrl;
    private final String repo;
    private final String token;
    private final Http http;
    private final LongSupplier clock;
    /** state → when it stops being accepted */
    private final Map<String, Long> pending = new ConcurrentHashMap<>();
    /** state → who it signed in, so naming the new account doesn't need GitHub's one-use code again */
    private final Map<String, Identity> identities = new ConcurrentHashMap<>();
    /**
     * state → the secret a naming retry must send in place of the code: the state travels in the callback URL
     * (browser history, proxy logs), so it alone must never be enough to finish someone else's sign-in
     */
    private final Map<String, String> resumes = new ConcurrentHashMap<>();
    private volatile Set<String> people = Collections.emptySet();
    private volatile long syncedAt;
    private volatile String syncProblem;

    public GithubAuth(String clientId, String clientSecret, String publicUrl, String repo, String token, Http http, LongSupplier clock) {
        this.clientId = trim(clientId);
        this.clientSecret = trim(clientSecret);
        this.publicUrl = trim(publicUrl).replaceAll("/+$", "");
        this.repo = trim(repo);
        this.token = trim(token);
        this.http = http;
        this.clock = clock;
    }

    public static GithubAuth get() {
        GithubAuth current = instance;
        if (current == null) {
            synchronized (GithubAuth.class) {
                current = instance;
                if (current == null) {
                    current = new GithubAuth(System.getProperty(CLIENT_ID_PROP, ""), System.getProperty(CLIENT_SECRET_PROP, ""),
                            System.getProperty(PUBLIC_URL_PROP, ""), System.getProperty(REPO_PROP, ""),
                            System.getProperty(TOKEN_PROP, ""), GithubAuth::httpRequest, System::currentTimeMillis);
                    current.startSync();
                    instance = current;
                }
            }
        }
        return current;
    }

    /** For tests. */
    public static void use(GithubAuth auth) {
        instance = auth;
    }

    public boolean isEnabled() {
        return !clientId.isEmpty() && !clientSecret.isEmpty() && !publicUrl.isEmpty();
    }

    public String repo() {
        return repo;
    }

    public long syncedAt() {
        return syncedAt;
    }

    /** why the last sync failed, or null */
    public String syncProblem() {
        return syncProblem;
    }

    public int peopleCount() {
        return people.size();
    }

    // --- sign-in ----------------------------------------------------------------------------------

    /**
     * Where to send the player: GitHub's authorization page, with a one-use state that comes back to us.
     */
    public String start() {
        if (!isEnabled()) {
            throw new IllegalStateException("Sign in with GitHub isn't set up on this server");
        }
        long now = clock.getAsLong();
        prune(now);
        if (pending.size() >= MAX_PENDING) {
            throw new IllegalStateException("Too many GitHub sign-ins at once. Try again in a few minutes.");
        }
        byte[] bytes = new byte[18];
        random.nextBytes(bytes);
        String state = Base64.getUrlEncoder().withoutPadding().encodeToString(bytes);
        pending.put(state, now + STATE_MILLIS);
        return "https://github.com/login/oauth/authorize?client_id=" + encode(clientId)
                + "&redirect_uri=" + encode(publicUrl + CALLBACK_PATH)
                + "&scope=" + encode("read:user user:email")
                + "&state=" + encode(state)
                + "&allow_signup=false";
    }

    /**
     * Who came back from GitHub with this state and code. The code is used once; asking again with the same state
     * (to pick an account name) returns the same person when it sends {@link #resumeSecret} in place of the code.
     */
    public Identity identify(String state, String code) throws IOException {
        long now = clock.getAsLong();
        prune(now);
        Identity known = state == null ? null : identities.get(state);
        if (known != null) {
            String expected = resumes.get(state);
            if (expected == null || code == null || !MessageDigest.isEqual(
                    expected.getBytes(StandardCharsets.UTF_8), code.trim().getBytes(StandardCharsets.UTF_8))) {
                throw new IllegalArgumentException("That GitHub sign-in has expired. Start again.");
            }
            return known;
        }
        Long until = state == null ? null : pending.remove(state);
        if (until == null || until < now) {
            throw new IllegalArgumentException("That GitHub sign-in has expired. Start again.");
        }
        if (code == null || code.trim().isEmpty()) {
            throw new IllegalArgumentException("GitHub didn't send a sign-in code. Start again.");
        }
        String form = "client_id=" + encode(clientId) + "&client_secret=" + encode(clientSecret) + "&code=" + encode(code.trim())
                + "&redirect_uri=" + encode(publicUrl + CALLBACK_PATH);
        JsonObject exchange = JsonParser.parseString(http.request("POST", "https://github.com/login/oauth/access_token", null, form)).getAsJsonObject();
        if (!exchange.has("access_token")) {
            throw new IllegalArgumentException("GitHub refused the sign-in. Start again.");
        }
        String access = exchange.get("access_token").getAsString();
        JsonObject user = JsonParser.parseString(http.request("GET", "https://api.github.com/user", access, null)).getAsJsonObject();
        String email = null;
        try {
            JsonArray emails = JsonParser.parseString(http.request("GET", "https://api.github.com/user/emails", access, null)).getAsJsonArray();
            for (JsonElement element : emails) {
                JsonObject entry = element.getAsJsonObject();
                if (entry.get("verified").getAsBoolean() && (email == null || entry.get("primary").getAsBoolean())) {
                    email = entry.get("email").getAsString();
                }
            }
        } catch (IOException | RuntimeException e) {
            logger.info("GitHub emails not readable for " + user.get("login").getAsString());
        }
        Identity identity = new Identity(user.get("id").getAsLong(), user.get("login").getAsString(), email);
        identities.put(state, identity);
        resumes.put(state, newSecret());
        pending.put("identity:" + state, now + STATE_MILLIS); // how long the identity is kept
        return identity;
    }

    /**
     * The secret a naming retry for this state sends as its code (null once the sign-in is done or forgotten).
     * It goes back to the client in the naming answer only, never in a URL.
     */
    public String resumeSecret(String state) {
        return state == null ? null : resumes.get(state);
    }

    /** The state's sign-in is finished, refused or failed: forget who it was. */
    public void done(String state) {
        if (state != null) {
            identities.remove(state);
            resumes.remove(state);
            pending.remove("identity:" + state);
        }
    }

    private static String newSecret() {
        byte[] bytes = new byte[18];
        random.nextBytes(bytes);
        return Base64.getUrlEncoder().withoutPadding().encodeToString(bytes);
    }

    private void prune(long now) {
        for (Iterator<Map.Entry<String, Long>> it = pending.entrySet().iterator(); it.hasNext(); ) {
            Map.Entry<String, Long> entry = it.next();
            if (entry.getValue() < now) {
                it.remove();
                if (entry.getKey().startsWith("identity:")) {
                    String state = entry.getKey().substring("identity:".length());
                    identities.remove(state);
                    resumes.remove(state);
                }
            }
        }
    }

    /**
     * A name the account could have here: GitHub logins allow dashes and 39 characters, account names don't.
     */
    public static String suggestName(String login) {
        String name = login == null ? "" : login.replaceAll("[^A-Za-z0-9_]", "_");
        if (name.length() > 14) {
            name = name.substring(0, 14);
        }
        while (name.length() < 3) {
            name = name + "_";
        }
        return name;
    }

    // --- the repository's people ------------------------------------------------------------------

    public boolean isRepoPerson(String login) {
        return login != null && people.contains(login.toLowerCase(Locale.ROOT));
    }

    private void startSync() {
        if (repo.isEmpty()) {
            return;
        }
        if (!repo.matches("[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")) {
            logger.warn(REPO_PROP + " should look like owner/name; not syncing its people");
            return;
        }
        ScheduledExecutorService executor = Executors.newSingleThreadScheduledExecutor(runnable -> {
            Thread thread = new Thread(runnable, "XMAGE github sync");
            thread.setDaemon(true);
            return thread;
        });
        executor.scheduleWithFixedDelay(this::syncNow, 0, SYNC_HOURS, TimeUnit.HOURS);
    }

    /**
     * Fetches the repository's collaborators (with a token) or public contributors (without).
     */
    public void syncNow() {
        boolean collaborators = !token.isEmpty();
        String what = collaborators ? "collaborators" : "contributors";
        Set<String> found = new HashSet<>();
        try {
            for (int page = 1; page <= 20; page++) {
                String body = http.request("GET", "https://api.github.com/repos/" + repo + "/" + what + "?per_page=100&page=" + page,
                        collaborators ? token : null, null);
                JsonElement parsed = body == null || body.trim().isEmpty() ? new JsonArray() : JsonParser.parseString(body);
                JsonArray list = parsed.isJsonArray() ? parsed.getAsJsonArray() : new JsonArray();
                for (JsonElement element : list) {
                    JsonObject person = element.getAsJsonObject();
                    if (person.has("login") && !"Bot".equals(person.has("type") ? person.get("type").getAsString() : "")) {
                        found.add(person.get("login").getAsString().toLowerCase(Locale.ROOT));
                    }
                }
                if (list.size() < 100) {
                    break;
                }
            }
            people = Collections.unmodifiableSet(found);
            syncedAt = clock.getAsLong();
            syncProblem = null;
            logger.info("GitHub: " + found.size() + " " + what + " of " + repo);
        } catch (IOException | RuntimeException e) {
            syncProblem = e.getClass().getSimpleName() + (e.getMessage() == null ? "" : ": " + e.getMessage());
            logger.warn("GitHub " + what + " of " + repo + " not fetched: " + syncProblem);
        }
    }

    // --- HTTP -------------------------------------------------------------------------------------

    static String httpRequest(String method, String url, String token, String formBody) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) new URL(url).openConnection();
        try {
            connection.setRequestMethod(method);
            connection.setConnectTimeout(5_000);
            connection.setReadTimeout(10_000);
            connection.setRequestProperty("Accept", url.startsWith("https://github.com/") ? "application/json" : "application/vnd.github+json");
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
                throw new IOException("GitHub answered HTTP " + status);
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
}
