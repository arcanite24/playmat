package mage.server;

import mage.server.managers.ConfigSettings;
import mage.server.managers.MailClient;
import org.apache.log4j.Logger;

import javax.mail.Message;
import javax.mail.MessagingException;
import javax.mail.Session;
import javax.mail.Transport;
import javax.mail.internet.InternetAddress;
import javax.mail.internet.MimeMessage;
import java.util.Properties;

/**
 * Mail by SMTP. Port 465 speaks TLS from the start (smtps); port 587, or XMAGE_MAIL_SMTP_STARTTLS=true on any port,
 * connects in plain text and must upgrade with STARTTLS before logging in (Purelymail, most providers' submission
 * port). Each mail gets its own session and properties, and a dead mail server gives up after
 * {@link #TIMEOUT_MS} instead of holding the calling thread.
 */
public class MailClientImpl implements MailClient {

    private static final Logger logger = Logger.getLogger(MailClientImpl.class);

    static final int TIMEOUT_MS = 10_000;
    public static final String STARTTLS_PROP = "xmage.mail.starttls";
    public static final String STARTTLS_ENV = "XMAGE_MAIL_SMTP_STARTTLS";

    private final ConfigSettings config;

    public MailClientImpl(ConfigSettings config) {
        this.config = config;
    }

    /** How to talk to the SMTP server: the transport ("smtp" or "smtps") and its properties. */
    static final class Smtp {
        final String protocol;
        final Properties properties;

        Smtp(String protocol, Properties properties) {
            this.protocol = protocol;
            this.properties = properties;
        }
    }

    static Smtp smtp(String host, String port, String user, boolean startTlsSetting) {
        String portText = port == null ? "" : port.trim();
        boolean startTls = startTlsSetting || "587".equals(portText);
        String protocol = startTls ? "smtp" : "smtps";
        String prefix = "mail." + protocol + '.';
        Properties properties = new Properties();
        properties.setProperty(prefix + "host", host == null ? "" : host.trim());
        if (!portText.isEmpty()) {
            properties.setProperty(prefix + "port", portText);
        }
        properties.setProperty(prefix + "auth", "true");
        properties.setProperty(prefix + "user", user == null ? "" : user.trim());
        properties.setProperty(prefix + "connectiontimeout", String.valueOf(TIMEOUT_MS));
        properties.setProperty(prefix + "timeout", String.valueOf(TIMEOUT_MS));
        properties.setProperty(prefix + "writetimeout", String.valueOf(TIMEOUT_MS));
        // the certificate must name the host, not just be valid
        properties.setProperty(prefix + "ssl.checkserveridentity", "true");
        // this JavaMail asks for TLSv1 unless told otherwise, which current JDKs refuse ("No appropriate protocol")
        properties.setProperty(prefix + "ssl.protocols", "TLSv1.3 TLSv1.2");
        if (startTls) {
            properties.setProperty(prefix + "starttls.enable", "true");
            // never log in over a connection that stayed plain text
            properties.setProperty(prefix + "starttls.required", "true");
        }
        return new Smtp(protocol, properties);
    }

    static boolean startTlsSetting() {
        return "true".equalsIgnoreCase(Mailer.setting(STARTTLS_PROP, STARTTLS_ENV));
    }

    public boolean sendMessage(String email, String subject, String text) {
        if (email == null || email.isEmpty()) {
            logger.info("Email is not sent because the address is empty");
            return false;
        }
        Smtp smtp = smtp(config.getMailSmtpHost(), config.getMailSmtpPort(), config.getMailUser(), startTlsSetting());
        Session session = Session.getInstance(smtp.properties);
        Transport transport = null;
        try {
            String from = config.getMailFromAddress();
            MimeMessage message = new MimeMessage(session);
            message.setFrom(new InternetAddress(from == null || from.trim().isEmpty() ? config.getMailUser() : from.trim()));
            message.addRecipient(Message.RecipientType.TO, new InternetAddress(email));
            message.setSubject(subject, "UTF-8");
            message.setText(text, "UTF-8");
            message.saveChanges();

            transport = session.getTransport(smtp.protocol);
            transport.connect(config.getMailUser(), config.getMailPassword());
            transport.sendMessage(message, message.getAllRecipients());
            return true;
        } catch (MessagingException ex) {
            logger.error("Error sending message to " + email + " by " + smtp.protocol, ex);
            return false;
        } finally {
            if (transport != null) {
                try {
                    transport.close();
                } catch (MessagingException ignored) {
                    // sent or not, the connection is done
                }
            }
        }
    }
}
