package mage.server.websocket.api;

import mage.server.websocket.rpc.RpcDispatcher;

/**
 * Web client bridge: every RPC method the web client can call.
 */
public final class WebClientApi {

    private WebClientApi() {
    }

    public static RpcDispatcher createDispatcher(ApiContext ctx) {
        RpcDispatcher dispatcher = new RpcDispatcher(ctx.sessions);
        dispatcher.registerAll(SessionApi.methods(ctx));
        dispatcher.registerAll(CatalogApi.methods(ctx));
        dispatcher.registerAll(DeckImportApi.methods(ctx));
        dispatcher.registerAll(LobbyApi.methods(ctx));
        dispatcher.registerAll(GameApi.methods(ctx));
        dispatcher.registerAll(EventApi.methods(ctx));
        dispatcher.registerAll(AdminApi.methods(ctx));
        dispatcher.registerAll(AccessApi.methods(ctx));
        dispatcher.registerAll(StaffApi.methods(ctx));
        dispatcher.registerAll(AccountsApi.methods(ctx));
        dispatcher.registerAll(SettingsApi.methods(ctx));
        dispatcher.registerAll(AccountApi.methods(ctx));
        dispatcher.registerAll(CareerApi.methods(ctx));
        dispatcher.registerAll(CareerProgressApi.methods(ctx));
        dispatcher.registerAll(CareerModesApi.methods(ctx));
        dispatcher.registerAll(SoloApi.methods(ctx));
        return dispatcher;
    }
}
