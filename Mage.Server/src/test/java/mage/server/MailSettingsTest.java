package mage.server;

import mage.server.managers.ConfigSettings;
import mage.server.managers.MailClient;
import mage.server.managers.ManagerFactory;
import org.junit.jupiter.api.Test;

import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Properties;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * How mail goes out, without a network: TLS from the start on 465, STARTTLS on 587 or when asked, timeouts always, the
 * JVM's own properties untouched, and SMTP before Mailgun.
 */
public class MailSettingsTest {

    @Test
    void port465SpeaksTlsFromTheStart() {
        MailClientImpl.Smtp smtp = MailClientImpl.smtp("smtp.purelymail.com", "465", "neri@example.test", false);
        assertThat(smtp.protocol).isEqualTo("smtps");
        assertThat(smtp.properties.getProperty("mail.smtps.host")).isEqualTo("smtp.purelymail.com");
        assertThat(smtp.properties.getProperty("mail.smtps.port")).isEqualTo("465");
        assertThat(smtp.properties.getProperty("mail.smtps.auth")).isEqualTo("true");
        assertThat(smtp.properties.getProperty("mail.smtps.starttls.enable")).isNull();
        assertThat(smtp.properties.getProperty("mail.smtps.ssl.checkserveridentity")).isEqualTo("true");
        assertThat(smtp.properties.getProperty("mail.smtps.ssl.protocols")).isEqualTo("TLSv1.3 TLSv1.2");
    }

    @Test
    void port587UpgradesWithStartTlsAndRequiresIt() {
        MailClientImpl.Smtp smtp = MailClientImpl.smtp("smtp.purelymail.com", " 587 ", "neri@example.test", false);
        assertThat(smtp.protocol).isEqualTo("smtp");
        assertThat(smtp.properties.getProperty("mail.smtp.port")).isEqualTo("587");
        assertThat(smtp.properties.getProperty("mail.smtp.starttls.enable")).isEqualTo("true");
        assertThat(smtp.properties.getProperty("mail.smtp.starttls.required")).isEqualTo("true");
        // STARTTLS on 587 failed against Purelymail with JavaMail's default TLSv1
        assertThat(smtp.properties.getProperty("mail.smtp.ssl.protocols")).isEqualTo("TLSv1.3 TLSv1.2");
        assertThat(smtp.properties.stringPropertyNames()).noneMatch(name -> name.startsWith("mail.smtps."));
    }

    @Test
    void theSettingTurnsStartTlsOnForAnyPort() {
        MailClientImpl.Smtp smtp = MailClientImpl.smtp("mail.example.test", "2525", "u", true);
        assertThat(smtp.protocol).isEqualTo("smtp");
        assertThat(smtp.properties.getProperty("mail.smtp.starttls.required")).isEqualTo("true");
    }

    @Test
    void aDeadServerTimesOut() {
        for (MailClientImpl.Smtp smtp : new MailClientImpl.Smtp[]{
                MailClientImpl.smtp("h", "465", "u", false), MailClientImpl.smtp("h", "587", "u", false)}) {
            String prefix = "mail." + smtp.protocol + '.';
            assertThat(smtp.properties.getProperty(prefix + "connectiontimeout")).isEqualTo("10000");
            assertThat(smtp.properties.getProperty(prefix + "timeout")).isEqualTo("10000");
        }
    }

    @Test
    void theJvmsPropertiesStayAsTheyWere() {
        Properties before = (Properties) System.getProperties().clone();
        MailClientImpl.smtp("smtp.example.test", "587", "u", false);
        assertThat(System.getProperty("mail.smtp.host")).isEqualTo(before.getProperty("mail.smtp.host"));
        assertThat(System.getProperty("mail.smtps.host")).isEqualTo(before.getProperty("mail.smtps.host"));
    }

    @Test
    void mailgunRegions() {
        assertThat(MailgunClientImpl.messagesUrl(null, "mg.example.test")).isEqualTo("https://api.mailgun.net/v3/mg.example.test/messages");
        assertThat(MailgunClientImpl.messagesUrl("https://api.eu.mailgun.net/v3/", "mg.example.test"))
                .isEqualTo("https://api.eu.mailgun.net/v3/mg.example.test/messages");
    }

    @Test
    void smtpGoesFirstThenMailgunThenNothing() {
        Map<String, String> config = new HashMap<>();
        List<String> sent = new ArrayList<>();
        ManagerFactory managers = managers(config, sent);

        config.put("getMailUser", "neri@example.test");
        config.put("getMailgunApiKey", "key");
        assertThat(Mailer.configured(managers.configSettings())).isTrue();
        assertThat(Mailer.send(managers, " pat@example.test ", "Hi", "text")).isTrue();
        assertThat(sent).containsExactly("smtp pat@example.test");

        sent.clear();
        config.put("getMailUser", "");
        assertThat(Mailer.send(managers, "pat@example.test", "Hi", "text")).isTrue();
        assertThat(sent).containsExactly("mailgun pat@example.test");

        sent.clear();
        config.put("getMailgunApiKey", "");
        assertThat(Mailer.configured(managers.configSettings())).isFalse();
        assertThat(Mailer.send(managers, "pat@example.test", "Hi", "text")).isFalse();
        assertThat(Mailer.send(managers, null, "Hi", "text")).isFalse();
        assertThat(sent).isEmpty();
    }

    private static ManagerFactory managers(Map<String, String> config, List<String> sent) {
        ConfigSettings settings = (ConfigSettings) Proxy.newProxyInstance(ConfigSettings.class.getClassLoader(),
                new Class<?>[]{ConfigSettings.class}, (proxy, method, args) -> config.getOrDefault(method.getName(), ""));
        MailClient smtp = (email, subject, text) -> sent.add("smtp " + email);
        MailClient mailgun = (email, subject, text) -> sent.add("mailgun " + email);
        return (ManagerFactory) Proxy.newProxyInstance(ManagerFactory.class.getClassLoader(), new Class<?>[]{ManagerFactory.class},
                (proxy, method, args) -> {
                    switch (method.getName()) {
                        case "configSettings":
                            return settings;
                        case "mailClient":
                            return smtp;
                        case "mailgunClient":
                            return mailgun;
                        default:
                            return null;
                    }
                });
    }
}
