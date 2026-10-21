package mage.server.websocket;

import java.net.InetAddress;
import java.net.UnknownHostException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.regex.Pattern;

/**
 * Web client bridge: finds the real client IP of a WebSocket connection.
 * <p>
 * Behind a reverse proxy the TCP peer is always the proxy, so the forwarding headers are used instead, but only when
 * the peer is a configured trusted proxy (otherwise anyone could claim any address). The client is the right-most
 * address in {@code X-Forwarded-For} that is not a trusted proxy; {@code X-Real-IP} is used when there is no
 * {@code X-Forwarded-For}. With an empty trusted list (the default) the TCP peer is always the client.
 * <p>
 * Configured with {@value #TRUSTED_PROXIES_PROP}: a comma separated list of IPs and CIDR ranges,
 * e.g. {@code 127.0.0.1,::1,172.16.0.0/12}.
 */
public final class ClientAddressResolver {

    static final String TRUSTED_PROXIES_PROP = "xmage.web.trustedProxies";

    private static final Pattern IPV4 = Pattern.compile("\\d{1,3}(\\.\\d{1,3}){3}");
    private static final Pattern IPV6 = Pattern.compile("[0-9a-fA-F:.]+(%[0-9a-zA-Z_.-]+)?");

    private final List<Cidr> trustedProxies;

    ClientAddressResolver(String trustedProxies) {
        this.trustedProxies = parseRanges(trustedProxies);
    }

    public static ClientAddressResolver fromSystemProperties() {
        return new ClientAddressResolver(System.getProperty(TRUSTED_PROXIES_PROP, ""));
    }

    boolean hasTrustedProxies() {
        return !trustedProxies.isEmpty();
    }

    /**
     * @param peer          IP address of the TCP peer
     * @param forwardedFor  X-Forwarded-For header value, may be null
     * @param realIp        X-Real-IP header value, may be null
     * @return the client IP address as text (normalized when it is a valid address)
     */
    public String resolve(String peer, String forwardedFor, String realIp) {
        InetAddress peerAddress = parseAddress(peer);
        if (peerAddress == null) {
            return peer == null ? "" : peer;
        }
        String client = peerAddress.getHostAddress();
        if (!isTrusted(peerAddress)) {
            return client;
        }

        if (forwardedFor != null && !forwardedFor.trim().isEmpty()) {
            String[] hops = forwardedFor.split(",");
            for (int i = hops.length - 1; i >= 0; i--) {
                InetAddress hop = parseAddress(hops[i]);
                if (hop == null) {
                    // garbage in the chain: the last trustworthy hop is the best we know
                    return client;
                }
                client = hop.getHostAddress();
                if (!isTrusted(hop)) {
                    return client;
                }
            }
            // every hop is a trusted proxy: the left-most one is the closest to the client
            return client;
        }

        if (realIp != null && !realIp.trim().isEmpty()) {
            InetAddress real = parseAddress(realIp);
            if (real != null) {
                return real.getHostAddress();
            }
        }
        return client;
    }

    boolean isTrusted(InetAddress address) {
        for (Cidr range : trustedProxies) {
            if (range.contains(address)) {
                return true;
            }
        }
        return false;
    }

    /**
     * Parses an IP literal without ever doing a DNS lookup. Accepts {@code [v6]}, {@code [v6]:port} and {@code v4:port}.
     *
     * @return the address or null when the text is not an IP literal
     */
    static InetAddress parseAddress(String text) {
        if (text == null) {
            return null;
        }
        String value = text.trim();
        if (value.startsWith("[")) {
            int end = value.indexOf(']');
            if (end < 0) {
                return null;
            }
            value = value.substring(1, end);
        } else if (value.indexOf(':') > 0 && value.indexOf(':') == value.lastIndexOf(':') && value.indexOf('.') > 0) {
            value = value.substring(0, value.indexOf(':')); // v4:port
        }
        if (value.isEmpty()) {
            return null;
        }
        boolean v4 = IPV4.matcher(value).matches();
        boolean v6 = !v4 && value.contains(":") && IPV6.matcher(value).matches();
        if (!v4 && !v6) {
            return null;
        }
        if (v4) {
            for (String part : value.split("\\.")) {
                if (Integer.parseInt(part) > 255) {
                    return null;
                }
            }
        }
        try {
            return InetAddress.getByName(value); // a literal, so no lookup happens
        } catch (UnknownHostException | RuntimeException e) {
            return null;
        }
    }

    static List<Cidr> parseRanges(String list) {
        if (list == null || list.trim().isEmpty()) {
            return Collections.emptyList();
        }
        List<Cidr> ranges = new ArrayList<>();
        for (String item : list.split(",")) {
            String value = item.trim();
            if (value.isEmpty()) {
                continue;
            }
            ranges.add(Cidr.parse(value));
        }
        return Collections.unmodifiableList(ranges);
    }

    static final class Cidr {
        private final byte[] network;
        private final int prefix;

        private Cidr(byte[] network, int prefix) {
            this.network = network;
            this.prefix = prefix;
        }

        static Cidr parse(String text) {
            int slash = text.indexOf('/');
            InetAddress address = parseAddress(slash < 0 ? text : text.substring(0, slash));
            if (address == null) {
                throw new IllegalArgumentException("Invalid trusted proxy address in " + TRUSTED_PROXIES_PROP + ": " + text);
            }
            int bits = address.getAddress().length * 8;
            int prefix = bits;
            if (slash >= 0) {
                try {
                    prefix = Integer.parseInt(text.substring(slash + 1).trim());
                } catch (NumberFormatException e) {
                    prefix = -1;
                }
                if (prefix < 0 || prefix > bits) {
                    throw new IllegalArgumentException("Invalid trusted proxy prefix in " + TRUSTED_PROXIES_PROP + ": " + text);
                }
            }
            return new Cidr(address.getAddress(), prefix);
        }

        boolean contains(InetAddress address) {
            byte[] candidate = address.getAddress();
            if (candidate.length != network.length) {
                // IPv4 range vs IPv6 address or the other way around (IPv4-mapped IPv6 is parsed as IPv4 already)
                return false;
            }
            int fullBytes = prefix / 8;
            for (int i = 0; i < fullBytes; i++) {
                if (candidate[i] != network[i]) {
                    return false;
                }
            }
            int remaining = prefix % 8;
            if (remaining == 0) {
                return true;
            }
            int mask = (0xFF << (8 - remaining)) & 0xFF;
            return (candidate[fullBytes] & mask) == (network[fullBytes] & mask);
        }
    }
}
