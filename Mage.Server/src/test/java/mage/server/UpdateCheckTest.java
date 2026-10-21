package mage.server;

import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

import static org.assertj.core.api.Assertions.assertThat;

/** The admin console's update check, against a stand-in registry and GitHub. */
public class UpdateCheckTest {

    private static final String OURS = "1111111111111111111111111111111111111111";
    private static final String NEWEST = "2222222222222222222222222222222222222222";

    private final List<String> calls = new ArrayList<>();
    private long now = 1_000_000L;
    private String registryCommit = NEWEST;
    private boolean githubDown;

    private String answer(String url, Map<String, String> headers) throws IOException {
        calls.add(url);
        if (url.startsWith("https://ghcr.io/token")) {
            return "{\"token\":\"t\"}";
        }
        if (url.startsWith("https://ghcr.io/v2/")) {
            assertThat(headers).containsEntry("Authorization", "Bearer t");
            String tag = url.substring(url.lastIndexOf('/') + 1);
            String version = "latest".equals(tag) ? "v1.3.0" : "sha-" + registryCommit.substring(0, 7);
            return "{\"annotations\":{\"org.opencontainers.image.revision\":\"" + registryCommit + "\","
                    + "\"org.opencontainers.image.version\":\"" + version + "\"}}";
        }
        if (url.startsWith("https://api.github.com/repos/arcanite24/playmat/compare/")) {
            if (githubDown) {
                throw new IOException("rate limited (HTTP 403), try again later");
            }
            return "{\"ahead_by\":3,\"html_url\":\"https://github.com/arcanite24/playmat/compare/x...y\",\"commits\":["
                    + commit("a", "Fix the stack's arrows") + ","
                    + commit("b", "Merge branch 'master' into thing") + ","
                    + commit("c", "Merge pull request #81 from arcanite24/thing\\n\\nMatch: a clearer combat step") + "]}";
        }
        throw new IOException("unexpected " + url);
    }

    private static String commit(String sha, String message) {
        return "{\"sha\":\"" + sha + "\",\"html_url\":\"https://github.com/c/" + sha + "\",\"commit\":{\"message\":\"" + message + "\"}}";
    }

    private UpdateCheck check(UpdateCheck.Build build) {
        return new UpdateCheck(build, true, this::answer, () -> now);
    }

    private static UpdateCheck.Build master(String commit) {
        return new UpdateCheck.Build("sha-" + commit.substring(0, 7), "master", "ghcr.io/arcanite24/playmat", "arcanite24/playmat", commit);
    }

    @Test
    public void masterBehindListsTheNewCommitsNewestFirst() {
        UpdateCheck.UpdateStatus status = check(master(OURS)).status(false);
        assertThat(status.error).isNull();
        assertThat(status.published).isTrue();
        assertThat(status.updateAvailable).isTrue();
        assertThat(status.commitsBehind).isEqualTo(3);
        assertThat(status.latestCommit).isEqualTo(NEWEST);
        assertThat(status.latestVersion).isEqualTo("sha-2222222");
        assertThat(status.serverImage).isEqualTo("ghcr.io/arcanite24/playmat-server:master");
        assertThat(status.webImage).isEqualTo("ghcr.io/arcanite24/playmat-web:master");
        assertThat(status.changes).extracting(c -> c.title).containsExactly("Match: a clearer combat step (#81)", "Fix the stack's arrows");
        assertThat(calls).anyMatch(url -> url.endsWith("/manifests/master"));
    }

    @Test
    public void upToDateAsksGithubNothing() {
        registryCommit = OURS;
        UpdateCheck.UpdateStatus status = check(master(OURS)).status(false);
        assertThat(status.updateAvailable).isFalse();
        assertThat(calls).noneMatch(url -> url.contains("api.github.com"));
    }

    @Test
    public void releaseFollowsLatestAndLinksTheReleasePage() {
        UpdateCheck.Build build = new UpdateCheck.Build("v1.2.0", "release", "ghcr.io/arcanite24/playmat", "arcanite24/playmat", OURS);
        UpdateCheck.UpdateStatus status = check(build).status(false);
        assertThat(calls).anyMatch(url -> url.endsWith("/manifests/latest"));
        assertThat(status.latestVersion).isEqualTo("v1.3.0");
        assertThat(status.notesUrl).isEqualTo("https://github.com/arcanite24/playmat/releases/tag/v1.3.0");
        assertThat(status.serverImage).isEqualTo("ghcr.io/arcanite24/playmat-server:latest");
    }

    @Test
    public void aLocalBuildIsComparedWithMaster() {
        UpdateCheck.Build build = new UpdateCheck.Build("", "", "", "", OURS);
        UpdateCheck.UpdateStatus status = check(build).status(false);
        assertThat(status.published).isFalse();
        assertThat(status.currentVersion).isNull();
        assertThat(status.channel).isEqualTo("master");
        assertThat(status.updateAvailable).isTrue();
        assertThat(status.serverImage).isEqualTo("ghcr.io/arcanite24/playmat-server:master");
    }

    @Test
    public void githubDownStillOffersTheUpdate() {
        githubDown = true;
        UpdateCheck.UpdateStatus status = check(master(OURS)).status(false);
        assertThat(status.updateAvailable).isTrue();
        assertThat(status.commitsBehind).isNull();
        assertThat(status.error).isNull();
        assertThat(status.notesUrl).isEqualTo("https://github.com/arcanite24/playmat/compare/" + OURS + "..." + NEWEST);
    }

    @Test
    public void answersAreKeptAndRefreshIsThrottled() {
        UpdateCheck check = check(master(OURS));
        check.status(false);
        int first = calls.size();
        now += 30_000;
        check.status(true);
        assertThat(calls).hasSize(first);
        now += UpdateCheck.REFRESH_MILLIS;
        check.status(true);
        assertThat(calls.size()).isGreaterThan(first);
        int second = calls.size();
        now += UpdateCheck.KEEP_MILLIS - 1;
        check.status(false);
        assertThat(calls).hasSize(second);
    }

    @Test
    public void offMeansNoCalls() {
        UpdateCheck.UpdateStatus status = new UpdateCheck(master(OURS), false, this::answer, () -> now).status(true);
        assertThat(status.enabled).isFalse();
        assertThat(status.updateAvailable).isNull();
        assertThat(calls).isEmpty();
    }

    @Test
    public void aBranchBuildFollowsNothing() {
        UpdateCheck.Build build = new UpdateCheck.Build("sha-1111111", "", "ghcr.io/arcanite24/playmat", "arcanite24/playmat", OURS);
        UpdateCheck.UpdateStatus status = check(build).status(false);
        assertThat(status.error).contains("follows nothing");
        assertThat(calls).isEmpty();
    }
}
