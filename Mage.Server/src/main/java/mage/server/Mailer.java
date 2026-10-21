package mage.server;

import mage.server.managers.ConfigSettings;
import mage.server.managers.ManagerFactory;
import org.apache.log4j.Logger;

/**
 * Mail to players: by SMTP when a mail user is set (XMAGE_MAIL_USER), else by Mailgun when its key is, else not at
 * all. Every place that mails a player goes through here, so they all pick the same way.
 */
public final class Mailer {

    private static final Logger logger = Logger.getLogger(Mailer.class);

    private Mailer() {
    }

    /** Whether the server can send mail at all. */
    public static boolean configured(ConfigSettings config) {
        return config != null && (smtp(config) || !isBlank(config.getMailgunApiKey()));
    }

    /**
     * @return true when the mail went out; false when it failed or the server has no mail
     */
    public static boolean send(ManagerFactory managers, String to, String subject, String text) {
        if (managers == null || to == null || to.trim().isEmpty()) {
            return false;
        }
        ConfigSettings config = managers.configSettings();
        if (smtp(config)) {
            return managers.mailClient().sendMessage(to.trim(), subject, text);
        }
        if (!isBlank(config.getMailgunApiKey())) {
            return managers.mailgunClient().sendMessage(to.trim(), subject, text);
        }
        logger.info("No mail sent (\"" + subject + "\"): the server has no mail settings");
        return false;
    }

    private static boolean smtp(ConfigSettings config) {
        return !isBlank(config.getMailUser());
    }

    /** A setting from a system property, else the environment variable; null when neither is set. */
    static String setting(String property, String environment) {
        String value = System.getProperty(property);
        if (value == null || value.trim().isEmpty()) {
            value = System.getenv(environment);
        }
        return value == null || value.trim().isEmpty() ? null : value.trim();
    }

    private static boolean isBlank(String text) {
        return text == null || text.trim().isEmpty();
    }
}
