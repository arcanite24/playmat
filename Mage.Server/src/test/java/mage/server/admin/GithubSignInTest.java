package mage.server.admin;

import mage.server.AuthorizedUserRepository;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * Sign in with GitHub: a one-use state, accounts found by link or verified email, new accounts named to fit, and a
 * contributors server that lets the repository's people in.
 */
public class GithubSignInTest {

    @TempDir
    Path dir;

    private final List<String> requests = new ArrayList<>();
    private long now = 1_000_000;
    private String user = "{\"id\": 42, \"login\": \"octo-cat-with-a-long-name\"}";
    private String emails = "[{\"email\": \"octo@example.test\", \"verified\": true, \"primary\": true}]";
    private String repoPeople = "[{\"login\": \"Octo-Cat-With-A-Long-Name\", \"type\": \"User\"}, {\"login\": \"dependabot\", \"type\": \"Bot\"}]";
    private AuthorizedUserRepository accounts;
    private AdminStore store;
    private GithubAuth github;

    @BeforeEach
    void setUp() throws Exception {
        accounts = new AuthorizedUserRepository("jdbc:h2:mem:github-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        github = new GithubAuth("client", "secret", "https://play.example.test/", "owner/repo", "", (method, url, token, form) -> {
            requests.add(method + " " + url);
            if (url.endsWith("/access_token")) {
                return form.contains("code=good") ? "{\"access_token\": \"t\"}" : "{\"error\": \"bad_verification_code\"}";
            }
            if (url.endsWith("/user")) {
                return user;
            }
            if (url.endsWith("/user/emails")) {
                return emails;
            }
            if (url.contains("/repos/owner/repo/contributors")) {
                return repoPeople;
            }
            throw new java.io.IOException("unexpected " + url);
        }, () -> now);
        GithubAuth.use(github);
    }

    @AfterEach
    void tearDown() {
        GithubAuth.use(null);
    }

    private String state() {
        Matcher matcher = Pattern.compile("state=([^&]+)").matcher(github.start());
        assertThat(matcher.find()).isTrue();
        return matcher.group(1);
    }

    private GithubSignIn signIn(AccessPolicy policy) {
        return new GithubSignIn(github, policy, accounts);
    }

    @Test
    void authorizeLinkComesBackToTheClient() {
        String url = github.start();

        assertThat(url).startsWith("https://github.com/login/oauth/authorize?client_id=client")
                .contains("redirect_uri=https%3A%2F%2Fplay.example.test%2Fauth%2Fgithub")
                .contains("scope=read%3Auser+user%3Aemail");
    }

    @Test
    void newAccountGetsANameThatFits() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        String state = state();

        GithubSignIn.GithubSignInResult first = signIn(open).finish(true, state, "good", null, now);

        assertThat(first.userName).isEqualTo("octo_cat_with_");
        assertThat(first.rememberToken).isNotEmpty();
        assertThat(accounts.isRememberedLogin("octo_cat_with_", first.rememberToken)).isTrue();
        assertThat(store.githubLinkOf("octo_cat_with_").login).isEqualTo("octo-cat-with-a-long-name");
    }

    @Test
    void stateWorksOnceAndExpires() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        String state = state();
        signIn(open).finish(true, state, "good", null, now);

        assertThatThrownBy(() -> signIn(open).finish(true, state, "good", null, now)).hasMessageContaining("expired");
        String late = state();
        now += GithubAuth.STATE_MILLIS + 1;
        assertThatThrownBy(() -> signIn(open).finish(true, late, "good", null, now)).hasMessageContaining("expired");
        assertThatThrownBy(() -> signIn(open).finish(true, "made-up", "good", null, now)).hasMessageContaining("expired");
    }

    @Test
    void takenNameAsksForAnother() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octo_cat_with_", "secret1", "someone@example.test");
        String state = state();

        GithubSignIn.GithubSignInResult ask = signIn(open).finish(true, state, "good", null, now);
        assertThat(ask.userName).isNull();
        assertThat(ask.suggestedName).isEqualTo("octo_cat_with_");

        assertThat(ask.resume).isNotBlank();
        assertThatThrownBy(() -> signIn(open).finish(true, state, null, "octo", now))
                .as("the state from the callback URL can't name the account on its own").hasMessageContaining("expired");
        assertThat(accounts.getByName("octo")).isNull();
    }

    @Test
    void theNamingRetryNeedsTheResumeSecret() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octo_cat_with_", "secret1", "someone@example.test");
        String state = state();

        GithubSignIn.GithubSignInResult ask = signIn(open).finish(true, state, "good", null, now);
        GithubSignIn.GithubSignInResult named = signIn(open).finish(true, state, ask.resume, "octo", now);
        assertThat(named.userName).isEqualTo("octo");
        assertThat(requests.stream().filter(r -> r.endsWith("/access_token"))).as("the code is used once").hasSize(1);
    }

    @Test
    void verifiedEmailSignsInToTheExistingAccount() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octavia", "secret1", "octo@example.test");
        store.emailVerified("octavia", "octo@example.test", now); // the account confirmed the address itself

        GithubSignIn.GithubSignInResult result = signIn(open).finish(true, state(), "good", null, now);

        assertThat(result.userName).isEqualTo("octavia");
        assertThat(store.githubLink(42).account).isEqualTo("octavia");
    }

    @Test
    void anAccountThatNeverConfirmedTheEmailIsNotTakenOver() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("squatter", "secret1", "octo@example.test"); // registered with someone else's address

        assertThatThrownBy(() -> signIn(open).finish(true, state(), "good", null, now))
                .hasMessage(GithubSignIn.UNCONFIRMED_EMAIL);
        assertThat(store.githubLink(42)).as("nothing is linked").isNull();
        assertThat(store.accountEmail("squatter")).as("nor vouched").isNull();
    }

    @Test
    void aRefusedLinkedSignInWritesNothing() throws Exception {
        AccessPolicy closed = AccessPolicy.create(store, "", "allowlist", "", now);
        accounts.add("octavia", "secret1", "octo@example.test");
        store.emailVerified("octavia", "octo@example.test", now);

        assertThatThrownBy(() -> signIn(closed).finish(true, state(), "good", null, now)).hasMessage(AccessPolicy.REFUSED);
        assertThat(store.githubLink(42)).isNull();
    }

    @Test
    void contributorsServerLetsTheRepositorysPeopleIn() throws Exception {
        AccessPolicy closed = AccessPolicy.create(store, "", "allowlist", "", now);
        assertThatThrownBy(() -> signIn(closed).finish(true, state(), "good", null, now)).hasMessage(AccessPolicy.REFUSED);

        github.syncNow();
        assertThat(github.isRepoPerson("octo-cat-with-a-long-name")).isTrue();
        assertThat(github.isRepoPerson("dependabot")).as("bots aren't people").isFalse();

        GithubSignIn.GithubSignInResult result = signIn(closed).finish(true, state(), "good", null, now);
        assertThat(result.userName).isEqualTo("octo_cat_with_");
        assertThat(closed.allows("octo_cat_with_", "octo@example.test")).isTrue();

        repoPeople = "[]";
        github.syncNow();
        store.disallow(store.allowlist().get(0).id);
        assertThat(closed.allows("octo_cat_with_", "octo@example.test")).as("off the repository and the list").isFalse();
    }

    @Test
    void refusedCodeAndMissingEmail() {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        assertThatThrownBy(() -> signIn(open).finish(true, state(), "bad", null, now)).hasMessageContaining("refused");

        emails = "[{\"email\": \"octo@example.test\", \"verified\": false, \"primary\": true}]";
        assertThatThrownBy(() -> signIn(open).finish(true, state(), "good", null, now)).hasMessageContaining("verified email");
        assertThatThrownBy(() -> signIn(open).finish(false, state(), "good", null, now)).hasMessageContaining("no accounts");
    }

    @Test
    void namesFitTheRules() {
        assertThat(GithubAuth.suggestName("a")).isEqualTo("a__");
        assertThat(GithubAuth.suggestName("x-y.z")).isEqualTo("x_y_z");
    }
}
