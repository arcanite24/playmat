package mage.server;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.apache.log4j.Logger;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.LongSupplier;

/**
 * Admin console: is a newer published image out than the one this server runs?
 * <p>
 * A published image (.github/workflows/images.yml) carries its tag, the channel it follows (master, or release for a
 * vX.Y.Z), the image names and the GitHub repository in its environment (Dockerfile.server). The check reads what the
 * channel's tag (master, or latest for releases) points to in the registry right now, from the annotations images.yml
 * writes on it, so it only ever offers something that can be pulled; GitHub's compare then counts the commits in
 * between and lists them. A local build carries none of that: it is compared with master's newest image, so its owner
 * sees what switching to published images would bring.
 * <p>
 * Nothing runs on its own: the first look in the admin console checks, and the answer is kept for an hour (a refresh
 * goes again, at most once a minute). Unauthenticated GitHub calls allow 60 an hour per address, plenty for that.
 * XMAGE_UPDATE_CHECK=off turns it off: no calls leave the server.
 */
public final class UpdateCheck {

    private static final Logger logger = Logger.getLogger(UpdateCheck.class);

    static final String ENABLED_ENV = "XMAGE_UPDATE_CHECK";
    static final String VERSION_ENV = "XMAGE_IMAGE_VERSION";
    static final String CHANNEL_ENV = "XMAGE_IMAGE_CHANNEL";
    static final String IMAGES_ENV = "XMAGE_IMAGE_NAMES";
    static final String REPO_ENV = "XMAGE_SOURCE_REPO";

    /** what a local build is compared with: this project's own images */
    static final String DEFAULT_IMAGES = "ghcr.io/arcanite24/playmat";
    static final String DEFAULT_REPO = "arcanite24/playmat";

    public static final String CHANNEL_MASTER = "master";
    public static final String CHANNEL_RELEASE = "release";

    static final long KEEP_MILLIS = 60 * 60 * 1000L;
    static final long REFRESH_MILLIS = 60 * 1000L;
    static final int TIMEOUT_MS = 8_000;
    /** the newest commits listed; the count covers all of them */
    static final int MAX_CHANGES = 20;

    private static final String INDEX_TYPES = "application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.list.v2+json";
    private static final String REVISION = "org.opencontainers.image.revision";
    private static final String VERSION = "org.opencontainers.image.version";

    /** GETs a URL and returns the body; for tests, without the network. */
    @FunctionalInterface
    interface Http {
        String get(String url, Map<String, String> headers) throws IOException;
    }

    /** What this server is: from the image's environment, and the commit Health reads. */
    static final class Build {
        final String version;
        final String channel;
        final String images;
        final String repo;
        final String commit;

        Build(String version, String channel, String images, String repo, String commit) {
            this.version = trim(version);
            this.channel = trim(channel);
            this.images = trim(images);
            this.repo = trim(repo);
            this.commit = trim(commit);
        }

        static Build fromEnvironment() {
            return new Build(System.getenv(VERSION_ENV), System.getenv(CHANNEL_ENV), System.getenv(IMAGES_ENV),
                    System.getenv(REPO_ENV), Health.commit());
        }

        boolean published() {
            return !version.isEmpty();
        }
    }

    /** One commit the update brings. */
    public static class UpdateChange {
        public String sha;
        public String title;
        public String url;
    }

    /** The admin console's Updates section. */
    public static class UpdateStatus {
        /** false when XMAGE_UPDATE_CHECK=off: nothing below was looked up */
        public boolean enabled;
        /** true for a published image; false for one built from a checkout */
        public boolean published;
        /** what the server follows: master or release; for a local build, master (what it is compared with) */
        public String channel;
        /** the image's tag (vX.Y.Z or sha-<7>), or null for a local build */
        public String currentVersion;
        public String currentCommit;
        public String latestVersion;
        public String latestCommit;
        /** null when it couldn't be told */
        public Boolean updateAvailable;
        /** commits the newest image has that this server doesn't; null when GitHub couldn't say */
        public Integer commitsBehind;
        /** newest first, at most {@value #MAX_CHANGES} */
        public List<UpdateChange> changes;
        /** the release page, or GitHub's comparison of the two commits */
        public String notesUrl;
        /** what to put in .env to follow the channel: XMAGE_SERVER_IMAGE and XMAGE_WEB_IMAGE */
        public String serverImage;
        public String webImage;
        public String sourceRepo;
        /** epoch millis of the last look, 0 when never */
        public long checkedAt;
        /** why the check failed, for staff; null when it worked */
        public String error;
        /** the Update button's updater, read fresh for every answer (the rest is kept for an hour) */
        public UpdateRunner.UpdaterInfo updater;

        /** this answer with the updater's state as it is now */
        public UpdateStatus withUpdater(UpdateRunner.UpdaterInfo info) {
            UpdateStatus copy = new UpdateStatus();
            copy.enabled = enabled;
            copy.published = published;
            copy.channel = channel;
            copy.currentVersion = currentVersion;
            copy.currentCommit = currentCommit;
            copy.latestVersion = latestVersion;
            copy.latestCommit = latestCommit;
            copy.updateAvailable = updateAvailable;
            copy.commitsBehind = commitsBehind;
            copy.changes = changes;
            copy.notesUrl = notesUrl;
            copy.serverImage = serverImage;
            copy.webImage = webImage;
            copy.sourceRepo = sourceRepo;
            copy.checkedAt = checkedAt;
            copy.error = error;
            copy.updater = info;
            return copy;
        }
    }

    private static final UpdateCheck INSTANCE = new UpdateCheck(Build.fromEnvironment(),
            !"off".equalsIgnoreCase(trim(System.getenv(ENABLED_ENV))), UpdateCheck::fetch, System::currentTimeMillis);

    public static UpdateCheck get() {
        return INSTANCE;
    }

    private final Build build;
    private final boolean enabled;
    private final Http http;
    private final LongSupplier clock;
    private UpdateStatus last;

    UpdateCheck(Build build, boolean enabled, Http http, LongSupplier clock) {
        this.build = build;
        this.enabled = enabled;
        this.http = http;
        this.clock = clock;
    }

    /** The kept answer while it's fresh; {@code refresh} looks again unless the last look was under a minute ago. */
    public synchronized UpdateStatus status(boolean refresh) {
        long now = clock.getAsLong();
        if (last != null) {
            long age = now - last.checkedAt;
            if (age < (refresh ? REFRESH_MILLIS : KEEP_MILLIS)) {
                return last;
            }
        }
        last = check(now);
        return last;
    }

    UpdateStatus check(long now) {
        UpdateStatus status = new UpdateStatus();
        status.enabled = enabled;
        status.published = build.published();
        status.channel = CHANNEL_RELEASE.equals(build.channel) ? CHANNEL_RELEASE : CHANNEL_MASTER;
        status.currentVersion = build.published() ? build.version : null;
        status.currentCommit = build.commit.isEmpty() ? null : build.commit;
        String images = build.images.isEmpty() ? DEFAULT_IMAGES : build.images;
        String tag = CHANNEL_RELEASE.equals(status.channel) ? "latest" : "master";
        status.serverImage = images + "-server:" + tag;
        status.webImage = images + "-web:" + tag;
        status.sourceRepo = build.repo.isEmpty() ? DEFAULT_REPO : build.repo;
        status.checkedAt = now;
        if (!enabled) {
            return status;
        }
        if (build.published() && build.channel.isEmpty()) {
            status.error = "This image was built from a branch other than master and follows nothing.";
            return status;
        }
        try {
            Map<String, String> annotations = annotations(images + "-server", tag);
            status.latestCommit = blankToNull(annotations.get(REVISION));
            status.latestVersion = blankToNull(annotations.get(VERSION));
        } catch (IOException | RuntimeException e) {
            logger.info("Update check: can't read " + status.serverImage + ": " + e.getMessage());
            status.error = "Can't read " + status.serverImage + " from the registry: " + e.getMessage();
            return status;
        }
        if (status.latestCommit == null) {
            status.error = status.serverImage + " doesn't say which commit it was built from.";
            return status;
        }
        if (status.currentCommit == null) {
            // a build without a commit: only the tag tells
            status.updateAvailable = status.currentVersion == null ? null : !status.currentVersion.equals(status.latestVersion);
        } else {
            status.updateAvailable = !status.latestCommit.equals(status.currentCommit);
        }
        if (CHANNEL_RELEASE.equals(status.channel) && status.latestVersion != null) {
            status.notesUrl = "https://github.com/" + status.sourceRepo + "/releases/tag/" + status.latestVersion;
        }
        if (Boolean.TRUE.equals(status.updateAvailable) && status.currentCommit != null) {
            compare(status);
        }
        return status;
    }

    /** Counts and lists the commits between the two; a failure leaves them unknown (the update is still offered). */
    private void compare(UpdateStatus status) {
        String url = "https://api.github.com/repos/" + status.sourceRepo + "/compare/" + status.currentCommit + "..." + status.latestCommit;
        try {
            JsonObject answer = JsonParser.parseString(http.get(url, githubHeaders())).getAsJsonObject();
            int ahead = answer.get("ahead_by").getAsInt();
            status.commitsBehind = ahead;
            // a server that runs something newer than the channel (a branch build) has nothing to update to
            status.updateAvailable = ahead > 0;
            if (status.notesUrl == null && answer.has("html_url")) {
                status.notesUrl = answer.get("html_url").getAsString();
            }
            status.changes = changes(answer.getAsJsonArray("commits"));
        } catch (IOException | RuntimeException e) {
            logger.info("Update check: GitHub's compare failed: " + e.getMessage());
            if (status.notesUrl == null) {
                status.notesUrl = "https://github.com/" + status.sourceRepo + "/compare/" + status.currentCommit + "..." + status.latestCommit;
            }
        }
    }

    /** GitHub lists them oldest first; merge commits stand for their pull request, by its title. */
    static List<UpdateChange> changes(JsonArray commits) {
        List<UpdateChange> changes = new ArrayList<>();
        if (commits == null) {
            return changes;
        }
        for (int i = commits.size() - 1; i >= 0 && changes.size() < MAX_CHANGES; i--) {
            JsonObject commit = commits.get(i).getAsJsonObject();
            String message = commit.getAsJsonObject("commit").get("message").getAsString();
            String[] lines = message.split("\\r?\\n");
            String title = lines[0].trim();
            if (title.startsWith("Merge pull request #")) {
                // "Merge pull request #80 from owner/branch", a blank line, then the pull request's title
                String number = title.substring("Merge pull request ".length()).split(" ")[0];
                String pullTitle = lines.length > 2 ? lines[2].trim() : "";
                title = pullTitle.isEmpty() ? title : pullTitle + " (" + number + ")";
            } else if (title.startsWith("Merge branch ") || title.startsWith("Merge remote-tracking branch ")) {
                continue;
            }
            UpdateChange change = new UpdateChange();
            change.sha = commit.get("sha").getAsString();
            change.title = title;
            JsonElement url = commit.get("html_url");
            change.url = url == null || url.isJsonNull() ? null : url.getAsString();
            changes.add(change);
        }
        return changes;
    }

    /** The annotations on a tag's image index: images.yml writes the commit and version there. GHCR only. */
    private Map<String, String> annotations(String image, String tag) throws IOException {
        int slash = image.indexOf('/');
        String registry = slash < 0 ? "" : image.substring(0, slash);
        String name = slash < 0 ? image : image.substring(slash + 1);
        if (!"ghcr.io".equals(registry)) {
            throw new IOException("only images on ghcr.io can be checked");
        }
        String token = JsonParser.parseString(http.get("https://ghcr.io/token?scope=repository:" + name + ":pull",
                Collections.emptyMap())).getAsJsonObject().get("token").getAsString();
        Map<String, String> headers = new LinkedHashMap<>();
        headers.put("Authorization", "Bearer " + token);
        headers.put("Accept", INDEX_TYPES);
        JsonObject index = JsonParser.parseString(http.get("https://ghcr.io/v2/" + name + "/manifests/" + tag, headers)).getAsJsonObject();
        Map<String, String> result = new LinkedHashMap<>();
        JsonObject annotations = index.getAsJsonObject("annotations");
        if (annotations != null) {
            for (Map.Entry<String, JsonElement> entry : annotations.entrySet()) {
                result.put(entry.getKey(), entry.getValue().getAsString());
            }
        }
        return result;
    }

    private static Map<String, String> githubHeaders() {
        Map<String, String> headers = new LinkedHashMap<>();
        headers.put("Accept", "application/vnd.github+json");
        return headers;
    }

    private static String fetch(String url, Map<String, String> headers) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) new URL(url).openConnection();
        try {
            connection.setConnectTimeout(TIMEOUT_MS);
            connection.setReadTimeout(TIMEOUT_MS);
            // GitHub refuses calls without one
            connection.setRequestProperty("User-Agent", "playmat-server/" + Health.version());
            for (Map.Entry<String, String> header : headers.entrySet()) {
                connection.setRequestProperty(header.getKey(), header.getValue());
            }
            int status = connection.getResponseCode();
            if (status == 404) {
                throw new IOException("not found");
            }
            if (status == 403 || status == 429) {
                throw new IOException("rate limited (HTTP " + status + "), try again later");
            }
            if (status != 200) {
                throw new IOException("HTTP " + status);
            }
            try (InputStream in = connection.getInputStream()) {
                ByteArrayOutputStream body = new ByteArrayOutputStream();
                byte[] buffer = new byte[8192];
                int read;
                // a compare lists up to 250 commits with their files: keep enough of it to parse
                while ((read = in.read(buffer)) != -1) {
                    if (body.size() > 16 * 1024 * 1024) {
                        throw new IOException("answer too large");
                    }
                    body.write(buffer, 0, read);
                }
                return new String(body.toByteArray(), StandardCharsets.UTF_8);
            }
        } finally {
            connection.disconnect();
        }
    }

    private static String trim(String text) {
        return text == null ? "" : text.trim();
    }

    private static String blankToNull(String text) {
        return text == null || text.trim().isEmpty() ? null : text.trim();
    }
}
