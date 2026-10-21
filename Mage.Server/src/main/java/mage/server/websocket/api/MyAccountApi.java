package mage.server.websocket.api;

import mage.server.AuthorizedUser;
import mage.server.AuthorizedUserRepository;
import mage.server.EmailVerification;
import mage.server.Mailer;
import mage.server.admin.AccessPolicy;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;

import java.util.Arrays;
import java.util.List;

import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.of;

/**
 * Web client bridge: the signed-in player's own account. Confirming its email address, downloading everything the
 * server keeps about it, and deleting it.
 */
final class MyAccountApi {

    private MyAccountApi() {
    }

    /** The account's email as Settings shows it. */
    public static class AccountEmailStatus {
        /** masked, like n***@example.com; empty when the account has none */
        public String email;
        /** the player proved they read it */
        public boolean verified;
        /** the server can send mail, so a code can be sent */
        public boolean mail;
        /** the account signs in with GitHub (it may have no password of its own) */
        public boolean github;
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("accountEmailStatus")
                        .returns("AccountEmailStatus")
                        .doc("The signed-in account's email, masked, and whether it is confirmed. Only on servers with accounts.")
                        .handler(call -> {
                            AuthorizedUser account = account(ctx, call);
                            AccountEmailStatus status = new AccountEmailStatus();
                            status.email = EmailVerification.mask(account.getEmail());
                            status.verified = EmailVerification.isVerified(account.getName(), account.getEmail());
                            status.mail = Mailer.configured(ctx.managers.configSettings());
                            status.github = AccessApi.store(() -> AccessPolicy.get().store().githubLinkOf(account.getName())) != null;
                            return status;
                        }),

                RpcMethod.named("accountEmailVerifyStart")
                        .returns("boolean")
                        .doc("Email a six-digit code that confirms the signed-in account's address (" + EmailVerification.SENDS_PER_HOUR
                                + " an hour, one a minute; each works 30 minutes and 5 tries). Fails with the reason.")
                        .handler(call -> {
                            AuthorizedUser account = account(ctx, call);
                            String problem = EmailVerification.get().sendCode(ctx.managers, account.getName(), System.currentTimeMillis());
                            if (problem != null) {
                                throw RpcException.invalidParams(problem);
                            }
                            return true;
                        }),

                RpcMethod.named("accountEmailVerify")
                        .params(of("code", STRING))
                        .returns("boolean")
                        .doc("Confirm the signed-in account's email with the mailed code. False when the code is wrong or has expired.")
                        .handler(call -> {
                            AuthorizedUser account = account(ctx, call);
                            return EmailVerification.get().verify(account.getName(), call.string(0), System.currentTimeMillis());
                        }),

                RpcMethod.named("accountExport")
                        .returns("string")
                        .doc("Everything the server keeps about the signed-in account as JSON, the same export staff can make: "
                                + "the account, decks, Career, reports, feedback, stats, support payments and perks. Passwords and sign-in tokens are left out.")
                        .handler(call -> {
                            AuthorizedUser account = account(ctx, call);
                            return AccessApi.store(() -> AccountsApi.export(account));
                        }),

                RpcMethod.named("accountDelete")
                        .params(of("password", STRING), of("confirmName", STRING))
                        .returns("boolean")
                        .doc("Delete the signed-in account with its decks, Career, reports, stats, feedback, staff role and "
                                + "supporter perks, and sign it out (its payments stay in the ledger, anonymized). confirmName must be the account's name exactly; password its password, except for "
                                + "an account that signs in with GitHub, which may leave it empty. The server's owner can't. "
                                + "Can't be undone: accountExport first.")
                        .handler(call -> {
                            AuthorizedUser account = account(ctx, call);
                            String name = account.getName();
                            refuseDeletion(ctx, account, call.string(0), call.string(1));
                            AccessApi.store(() -> {
                                AccountsApi.delete(ctx, name, "Your account was deleted. Thanks for playing.");
                                return null;
                            });
                            Staff.audit(name, "user.delete.self", name, null, null);
                            return true;
                        })
        );
    }

    /** guesses at the password through deletion, an hour */
    static final int DELETE_TRIES_PER_HOUR = 5;

    /** Throws why the account can't be deleted with this password and typed name. */
    static void refuseDeletion(ApiContext ctx, AuthorizedUser account, String password, String confirmName) throws RpcException {
        String name = account.getName();
        if (AccessPolicy.get().isOwner(name)) {
            throw RpcException.invalidParams("The server's owner (XMAGE_OWNER) can't delete their own account here.");
        }
        if (confirmName == null || !name.equals(confirmName.trim())) {
            throw RpcException.invalidParams("Type your account name, " + name + ", exactly as it is.");
        }
        if (!AccessApi.takeSlot("delete:" + name.toLowerCase(java.util.Locale.ROOT), System.currentTimeMillis(), DELETE_TRIES_PER_HOUR)) {
            throw new RpcException(RpcException.RATE_LIMITED, "Too many tries. Wait an hour.");
        }
        boolean github = AccessApi.store(() -> AccessPolicy.get().store().githubLinkOf(name)) != null;
        boolean passwordless = github && (password == null || password.isEmpty());
        if (!passwordless && (password == null || password.isEmpty() || !account.doCredentialsMatch(name, password))) {
            throw RpcException.invalidParams(github ? "That password is wrong. An account that signs in with GitHub may leave it empty."
                    : "That password is wrong.");
        }
    }

    /** The signed-in player's account; refused on servers without accounts. */
    static AuthorizedUser account(ApiContext ctx, RpcCall call) throws RpcException {
        String userName = ctx.sessions.userName(call.sessionId())
                .orElseThrow(() -> RpcException.notAuthorized("Sign in first"));
        if (!AccessApi.accounts(ctx)) {
            throw RpcException.invalidParams("This server has no accounts");
        }
        AuthorizedUser account = AuthorizedUserRepository.getInstance().getByName(userName);
        if (account == null) {
            throw RpcException.invalidParams("There is no account named " + userName);
        }
        return account;
    }
}
