package mage.server;

import mage.server.admin.AccessPolicy;
import mage.server.admin.AccountEmail;
import mage.server.admin.AdminStore;
import mage.server.admin.StaffRole;
import mage.server.managers.ManagerFactory;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.lang.reflect.Proxy;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Date;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.function.Function;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Email verification: the stored address, codes (expiry, tries, how often), a password reset by mailed code, and an
 * unverified address released to a new account after a while.
 */
public class EmailVerificationTest {

    private static final String PASSWORD = "correct horse 7";
    private static final long HOUR = 3_600_000L;

    @TempDir
    Path dir;

    private final Map<String, Function<Object[], Object>> overrides = new HashMap<>();
    private final List<String[]> mails = new ArrayList<>();
    private AuthorizedUserRepository previousAccounts;
    private AuthorizedUserRepository accounts;
    private AdminStore store;
    private ManagerFactory managers;

    @BeforeEach
    void setUp() throws Exception {
        previousAccounts = AuthorizedUserRepository.getInstance();
        accounts = new AuthorizedUserRepository("jdbc:h2:mem:verify-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        AuthorizedUserRepository.use(accounts);
        store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        AccessPolicy.use(AccessPolicy.create(store, "owner", "open", "", 1000));
        managers = deepProxy(ManagerFactory.class);
        overrides.put("isAuthenticationActivated", args -> true);
        overrides.put("getMinUserNameLength", args -> 3);
        overrides.put("getMaxUserNameLength", args -> 20);
        overrides.put("getMinPasswordLength", args -> 8);
        overrides.put("getMaxPasswordLength", args -> 100);
        overrides.put("getInvalidUserNamePattern", args -> "[^a-zA-Z0-9_-]");
        overrides.put("getMailUser", args -> "mailer@example.test");
        overrides.put("getMailgunApiKey", args -> "");
        overrides.put("getServerName", args -> "Playmat");
        overrides.put("sendMessage", args -> mails.add(new String[]{(String) args[0], (String) args[2]}));
    }

    @AfterEach
    void tearDown() {
        AuthorizedUserRepository.use(previousAccounts);
        AccessPolicy.use(null);
    }

    @Test
    void verifiedOnlyForTheAddressItProved() throws Exception {
        accounts.add("pat", PASSWORD, "pat@example.test");
        assertThat(EmailVerification.isVerified("pat")).isFalse();

        EmailVerification.markVerified("Pat", "Pat@Example.test", 10);
        assertThat(EmailVerification.isVerified("pat", "pat@example.test")).as("any case").isTrue();
        assertThat(EmailVerification.isVerified("pat")).isTrue();
        assertThat(EmailVerification.isVerified("pat", "other@example.test")).isFalse();

        accounts.changeEmail("pat", "new@example.test");
        assertThat(EmailVerification.isVerified("pat")).as("a changed email starts unverified").isFalse();

        store.forgetAccount("pat");
        assertThat(store.accountEmail("pat")).isNull();
    }

    @Test
    void registrationMailsAFirstCodeThatConfirmsTheAddress() throws Exception {
        assertThat(newSession().registerUser("pat", PASSWORD, "pat@example.test")).isNull();
        assertThat(mails).hasSize(1);
        String code = code(mails.get(0)[1]);

        assertThat(EmailVerification.get().verify("pat", "000000".equals(code) ? "111111" : "000000", now())).isFalse();
        assertThat(EmailVerification.get().verify("pat", code, now())).isTrue();
        assertThat(EmailVerification.isVerified("pat")).isTrue();
        assertThat(store.accountEmail("pat").registeredAt).isPositive();
    }

    @Test
    void codesExpireAndRunOutOfTries() {
        accounts.add("pat", PASSWORD, "pat@example.test");
        EmailVerification verification = new EmailVerification();

        String code = verification.issue("pat", "pat@example.test", 0);
        assertThat(verification.verify("pat", code, ResetTokens.VALID_MILLIS + 1)).as("expired").isFalse();

        code = verification.issue("pat", "pat@example.test", 2 * HOUR);
        String wrong = code.equals("000000") ? "000001" : "000000";
        for (int i = 0; i < ResetTokens.ATTEMPTS; i++) {
            assertThat(verification.verify("pat", wrong, 2 * HOUR + i)).isFalse();
        }
        assertThat(verification.verify("pat", code, 2 * HOUR + 10)).as("out of tries").isFalse();
        assertThat(EmailVerification.isVerified("pat")).isFalse();
    }

    @Test
    void aCodeOnlyConfirmsTheAddressItWentTo() {
        accounts.add("pat", PASSWORD, "pat@example.test");
        EmailVerification verification = new EmailVerification();
        String code = verification.issue("pat", "pat@example.test", 0);
        accounts.changeEmail("pat", "elsewhere@example.test");
        assertThat(verification.verify("pat", code, 1)).isFalse();
    }

    @Test
    void codesAreSentAtMostOnceAMinuteAndFiveTimesAnHour() {
        EmailVerification verification = new EmailVerification();
        verification.issue("pat", "pat@example.test", 0);
        assertThat(problem(() -> verification.issue("pat", "pat@example.test", 30_000))).contains("Wait a minute");
        for (int i = 1; i < EmailVerification.SENDS_PER_HOUR; i++) {
            verification.issue("pat", "pat@example.test", i * 61_000L);
        }
        assertThat(problem(() -> verification.issue("pat", "pat@example.test", 10 * 61_000L))).contains("this hour");
        assertThat(problem(() -> verification.issue("sam", "sam@example.test", 10 * 61_000L))).as("per account").isNull();
        assertThat(problem(() -> verification.issue("pat", "pat@example.test", HOUR + 5 * 61_000L))).as("an hour later").isNull();
    }

    @Test
    void sendingNeedsMailAndAnUnconfirmedAddress() {
        accounts.add("pat", PASSWORD, "pat@example.test");
        EmailVerification verification = new EmailVerification();
        assertThat(verification.sendCode(managers, "pat", 0)).isNull();
        assertThat(mails).hasSize(1);
        assertThat(mails.get(0)[0]).isEqualTo("pat@example.test");
        assertThat(verification.verify("pat", code(mails.get(0)[1]), 1)).isTrue();
        assertThat(verification.sendCode(managers, "pat", 2 * HOUR)).contains("confirmed already");

        accounts.add("sam", PASSWORD, "sam@example.test");
        overrides.put("getMailUser", args -> "");
        assertThat(verification.sendCode(managers, "sam", 0)).contains("can't send email");
    }

    @Test
    void aPasswordResetByMailedCodeConfirmsTheAddressAStaffCodeDoesnt() {
        accounts.add("pat", PASSWORD, "pat@example.test");
        AccountService service = new AccountService(managers);
        assertThat(service.sendResetCode("pat@example.test")).isNull();
        assertThat(service.resetPassword("pat@example.test", code(mails.get(0)[1]), "new horse 8")).isNull();
        assertThat(EmailVerification.isVerified("pat")).isTrue();

        accounts.add("sam", PASSWORD, "sam@example.test");
        AccountService.StaffResetCode staff = service.staffResetCode("sam");
        assertThat(service.resetPassword("sam@example.test", staff.code, "new horse 8")).isNull();
        assertThat(EmailVerification.isVerified("sam")).as("staff saw that code").isFalse();
    }

    @Test
    void anUnverifiedAddressIsReleasedToANewAccountAfterTwoDaysAway() throws Exception {
        long now = now();
        accounts.add("squat", PASSWORD, "owner@example.test");
        store.accountRegistered("squat", now - 72 * HOUR);
        lastSignIn("squat", now - 72 * HOUR);

        assertThat(newSession().registerUser("realowner", PASSWORD, "owner@example.test")).isNull();
        assertThat(accounts.getByName("squat").getEmail()).isNull();
        assertThat(accounts.getByEmail("owner@example.test").getName()).isEqualTo("realowner");
        assertThat(store.auditLog(null, "user.email.release", "squat", 0, 10)).hasSize(1);
    }

    @Test
    void anAddressStaysWithAnAccountThatIsNewActiveVerifiedOrStaff() throws Exception {
        long now = now();
        accounts.add("fresh", PASSWORD, "fresh@example.test");
        store.accountRegistered("fresh", now - HOUR);
        accounts.add("active", PASSWORD, "active@example.test");
        lastSignIn("active", now - HOUR);
        accounts.add("proved", PASSWORD, "proved@example.test");
        store.emailVerified("proved", "proved@example.test", 1);
        accounts.add("mod", PASSWORD, "mod@example.test");
        store.grantStaff("mod", StaffRole.MODERATOR, "owner", 1);

        for (String email : new String[]{"fresh@example.test", "active@example.test", "proved@example.test", "mod@example.test"}) {
            assertThat(newSession().registerUser("taker", PASSWORD, email)).as(email).contains("associated with another user");
        }

        accounts.add("legacy", PASSWORD, "legacy@example.test");
        assertThat(EmailVerification.releasable(accounts.getByName("legacy"), now))
                .as("registered before the times were kept, never signed in").isTrue();
    }

    @Test
    void squattedFollowsTheRules() {
        long now = 100 * HOUR;
        AccountEmail row = new AccountEmail();
        assertThat(EmailVerification.squatted("a@x.test", null, null, now)).isTrue();
        row.registeredAt = now - HOUR;
        assertThat(EmailVerification.squatted("a@x.test", row, null, now)).isFalse();
        row.registeredAt = now - 49 * HOUR;
        assertThat(EmailVerification.squatted("a@x.test", row, new Date(now - HOUR), now)).isFalse();
        assertThat(EmailVerification.squatted("a@x.test", row, new Date(now - 49 * HOUR), now)).isTrue();
        row.verifiedEmail = "A@x.test";
        assertThat(EmailVerification.squatted("a@x.test", row, null, now)).isFalse();
    }

    @Test
    void masks() {
        assertThat(EmailVerification.mask("neri@example.com")).isEqualTo("n***@example.com");
        assertThat(EmailVerification.mask(null)).isEmpty();
        assertThat(EmailVerification.mask("odd")).isEqualTo("o***");
    }

    private void lastSignIn(String name, long at) {
        AuthorizedUser user = accounts.getByName(name);
        user.lastConnection = new Date(at);
        accounts.update(user);
    }

    private static long now() {
        return System.currentTimeMillis();
    }

    private static String code(String text) {
        Matcher matcher = Pattern.compile("(\\d{6})").matcher(text);
        assertThat(matcher.find()).as("a code in: " + text).isTrue();
        return matcher.group(1);
    }

    private static String problem(Runnable work) {
        try {
            work.run();
            return null;
        } catch (IllegalStateException e) {
            return e.getMessage();
        }
    }

    private Session newSession() {
        Session session = new Session(managers, UUID.randomUUID().toString(), null);
        session.setHost("192.0.2.10");
        return session;
    }

    @SuppressWarnings("unchecked")
    private <T> T deepProxy(Class<T> type) {
        return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[]{type}, (proxy, method, args) -> {
            Function<Object[], Object> override = overrides.get(method.getName());
            if (override != null) {
                return override.apply(args == null ? new Object[0] : args);
            }
            Class<?> result = method.getReturnType();
            if (result == Optional.class) {
                return Optional.empty();
            }
            if (result == boolean.class) {
                return false;
            }
            if (result == int.class || result == long.class) {
                return result == int.class ? (Object) 0 : (Object) 0L;
            }
            if (result.isInterface()) {
                return deepProxy(result);
            }
            return null;
        });
    }
}
