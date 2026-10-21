package mage.server;

import com.sun.jersey.api.client.Client;
import com.sun.jersey.api.client.ClientHandlerException;
import com.sun.jersey.api.client.ClientResponse;
import com.sun.jersey.api.client.WebResource;
import com.sun.jersey.api.client.filter.HTTPBasicAuthFilter;
import com.sun.jersey.core.util.MultivaluedMapImpl;
import mage.server.managers.ConfigSettings;
import mage.server.managers.MailClient;
import org.apache.log4j.Logger;

import javax.ws.rs.core.MediaType;

/**
 * Mail through Mailgun's API. XMAGE_MAILGUN_BASE_URL picks the region: https://api.eu.mailgun.net/v3 for a domain
 * in Mailgun's EU region.
 */
public class MailgunClientImpl implements MailClient {

    private static final Logger logger = Logger.getLogger(MailgunClientImpl.class);

    public static final String DEFAULT_BASE_URL = "https://api.mailgun.net/v3";
    public static final String BASE_URL_PROP = "xmage.mailgun.baseUrl";
    public static final String BASE_URL_ENV = "XMAGE_MAILGUN_BASE_URL";

    private final ConfigSettings config;

    public MailgunClientImpl(ConfigSettings config) {
        this.config = config;
    }

    static String messagesUrl(String baseUrl, String domain) {
        String base = baseUrl == null || baseUrl.trim().isEmpty() ? DEFAULT_BASE_URL : baseUrl.trim();
        while (base.endsWith("/")) {
            base = base.substring(0, base.length() - 1);
        }
        return base + '/' + domain + "/messages";
    }

    public boolean sendMessage(String email, String subject, String text) {
        if (email == null || email.isEmpty()) {
            logger.info("Email is not sent because the address is empty");
            return false;
        }
        Client client = Client.create();
        // a dead API can't hold the calling thread
        client.setConnectTimeout(MailClientImpl.TIMEOUT_MS);
        client.setReadTimeout(MailClientImpl.TIMEOUT_MS);
        client.addFilter(new HTTPBasicAuthFilter("api", config.getMailgunApiKey()));
        String domain = config.getMailgunDomain();
        WebResource webResource = client.resource(messagesUrl(Mailer.setting(BASE_URL_PROP, BASE_URL_ENV), domain));
        MultivaluedMapImpl formData = new MultivaluedMapImpl();
        String from = config.getMailFromAddress();
        formData.add("from", from == null || from.trim().isEmpty() ? "XMage <postmaster@" + domain + '>' : from.trim());
        formData.add("to", email);
        formData.add("subject", subject);
        formData.add("text", text);
        try {
            ClientResponse response = webResource.type(MediaType.APPLICATION_FORM_URLENCODED).post(ClientResponse.class, formData);
            boolean succeeded = response.getStatus() == 200;
            if (!succeeded) {
                logger.error("Error sending message to " + email + ". Status code: " + response.getStatus());
            }
            return succeeded;
        } catch (ClientHandlerException e) {
            logger.error("Error sending message to " + email + ": Mailgun can't be reached (" + e.getMessage() + ')');
            return false;
        } finally {
            client.destroy();
        }
    }
}
