# XMage Web Client Documentation

This directory contains documentation for implementing a web client for XMage using the WebSocket API.

## Documentation Files

| File | Description |
|------|-------------|
| [WebSocketAPI.md](./WebSocketAPI.md) | Complete WebSocket API specification with all methods, parameters, and return types. |
| [WebClientDataModels.md](./WebClientDataModels.md) | TypeScript interface definitions for all data models used in the API. |
| [WebClientImplementationPlan.md](./WebClientImplementationPlan.md) | Detailed implementation plan with architecture, roadmap, and technical guidance. |
| [UpstreamMerge.md](./UpstreamMerge.md) | Monthly upstream merge recipe, conflict policy and the fork's edits outside its own paths. |
| [RustMigration.md](./RustMigration.md) | Investigation and phased plan for moving the server to Rust behind per-subsystem toggles. |

## Quick Start

### 1. Connect to the Server

```javascript
const ws = new WebSocket('ws://localhost:17172');

ws.onopen = () => {
  console.log('Connected to XMage server');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  
  // Check if it's a response to a request
  if (message.jsonrpc === '2.0' && message.id) {
    handleResponse(message);
  } else {
    // It's a server push (ClientCallback)
    handleCallback(message);
  }
};
```

### 2. Login

```javascript
// Generate a session ID (or restore from localStorage)
const sessionId = crypto.randomUUID();

// Send login request
ws.send(JSON.stringify({
  method: 'connectUser',
  params: ['myUsername', 'myPassword', sessionId, '', '', ''],
  id: 1
}));

// Set user preferences
ws.send(JSON.stringify({
  method: 'connectSetUserData',
  params: ['myUsername', sessionId, {
    groupId: 0,
    avatarId: 51,
    confirmEmptyManaPool: true,
    flagName: 'world.png',
    userSkipPrioritySteps: {
      yourTurn: { upkeep: true, draw: true, main1: false, beforeCombat: true, endOfCombat: true, main2: false, endOfTurn: true },
      opponentTurn: { upkeep: true, draw: true, main1: true, beforeCombat: true, endOfCombat: true, main2: true, endOfTurn: true },
      stopOnDeclareAttackers: true,
      stopOnDeclareBlockersWithAnyPermanents: true
    }
  }, '1.0.0', ''],
  id: 2
}));
```

### 3. Get Lobby Tables

```javascript
// Get main room ID
ws.send(JSON.stringify({
  method: 'serverGetMainRoomId',
  params: [],
  id: 3
}));

// After receiving roomId, get tables
ws.send(JSON.stringify({
  method: 'roomGetAllTables',
  params: [roomId],
  id: 4
}));
```

### 4. Handle Game Callbacks

```javascript
function handleCallback(callback) {
  switch (callback.method) {
    case 'gameInit':
    case 'gameUpdate':
    case 'gameInform':
      updateGameState(callback.data);
      break;
    
    case 'gameAsk':
      showYesNoDialog(callback.data);
      break;
    
    case 'gameTarget':
      enableTargetSelection(callback.data);
      break;
    
    case 'chatMessage':
      appendChatMessage(callback.data);
      break;
    
    case 'gameOver':
      showGameEndScreen(callback.data);
      break;
  }
}
```

## Backend Implementation

The WebSocket server is implemented in:

- **Server**: `Mage.Server/src/main/java/mage/server/websocket/WebSocketServerImpl.java`
- **Callback Handler**: `Mage.Server/src/main/java/mage/server/websocket/WebSocketCallbackHandler.java`

The server uses JSON-RPC 2.0 (simplified) over WebSockets and delegates to the existing `MageServer` interface for all game logic.

### Default Port

The WebSocket server runs on port **17172** by default.

## Key Concepts

### Session ID

Each client generates a UUID as their `sessionId`. This ID should be:
- Generated on first connection
- Stored in `localStorage` for reconnection
- Sent with most API calls for authentication

### UserData

User preferences (F-key settings, avatar, flag) are sent after login via `connectSetUserData`. These control auto-pass behavior and display preferences.

### ClientCallback

All server-to-client push messages use the `ClientCallback` format:

```typescript
interface ClientCallback {
  messageId: number;
  method: string;
  objectId: UUID | null;
  data: unknown;  // Type varies by method
}
```

### GameView

The `GameView` object is the main game state container. It includes:
- All player information (life, hand count, battlefield, etc.)
- Current phase/step
- Stack contents
- Combat groups
- Your hand cards (full details)

## Technology Recommendations

- **React + TypeScript** for the frontend
- **Zustand** for state management
- **Tailwind CSS** for styling
- **Zod** for runtime validation of server responses

See [WebClientImplementationPlan.md](./WebClientImplementationPlan.md) for detailed guidance.
