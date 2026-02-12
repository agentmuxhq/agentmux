# AgentMux Case-Insensitive Agent IDs

**Date:** 2026-01-16
**Author:** AgentA
**Status:** Proposed
**Priority:** High (blocking cross-host testing)

---

## Problem

Agent IDs are currently case-sensitive throughout AgentMux. This causes silent failures:

- Sending to "AgentG" vs "agentg" results in messages going to different destinations
- MCP mux messages fail silently when case doesn't match
- Reactive injections queue for wrong agent ID
- Users don't realize case matters, leading to debugging confusion

**Example failures today:**
```
mcp__agentmux__send_message to="AgentG"  → never delivered
mcp__agentmux__send_message to="agentg"  → works

/reactive/inject target_agent="AgentG"   → queued for "AgentG"
poller polls for "agentg"                → never fetches it
```

---

## Solution

Normalize all agent IDs to lowercase throughout the system.

### Changes Required

#### 1. AgentMux Server (Lambda)

**File:** `src/handlers/*.ts`

Normalize agent IDs on input:

```typescript
// Add helper function
function normalizeAgentId(agentId: string): string {
  return agentId.toLowerCase().trim();
}

// Apply to all handlers
export async function sendMessage(event) {
  const to = normalizeAgentId(event.body.to);
  const from = normalizeAgentId(event.headers['x-agent-id']);
  // ...
}

export async function injectReactive(event) {
  const targetAgent = normalizeAgentId(event.body.target_agent);
  const sourceAgent = normalizeAgentId(event.headers['x-agent-id']);
  // ...
}

export async function getPending(event) {
  const agentId = normalizeAgentId(event.pathParameters.agent_id);
  // ...
}
```

#### 2. DynamoDB Queries

Ensure GSI queries use normalized agent IDs:

```typescript
// When storing
await dynamodb.put({
  TableName: 'agentmux-messages',
  Item: {
    to: normalizeAgentId(to),        // Always lowercase
    from: normalizeAgentId(from),    // Always lowercase
    // ...
  }
});

// When querying
const result = await dynamodb.query({
  IndexName: 'to-timestamp-index',
  KeyConditionExpression: '#to = :to',
  ExpressionAttributeValues: {
    ':to': normalizeAgentId(agentId)  // Normalize query too
  }
});
```

#### 3. MCP Tools

**File:** `src/mcp/tools.ts`

Normalize in MCP tool handlers:

```typescript
export const sendMessageTool = {
  handler: async (params) => {
    const to = params.to.toLowerCase().trim();
    const message = params.message;
    // ...
  }
};
```

#### 4. AgentMux Poller (Optional)

For defense in depth, normalize in AgentMux too:

**File:** `pkg/reactive/poller.go`

```go
import "strings"

func (p *Poller) pollForAgent(agentID string) error {
    // Normalize agent ID
    agentID = strings.ToLower(strings.TrimSpace(agentID))

    reqURL := fmt.Sprintf("%s/reactive/pending/%s",
        p.agentmuxURL, url.PathEscape(agentID))
    // ...
}
```

---

## Migration

### Existing Data

For messages already in DynamoDB with mixed case:

1. **Option A (Simple):** Let them expire naturally (TTL)
2. **Option B (Complete):** One-time migration script to lowercase all agent IDs

### Backward Compatibility

- Old clients sending "AgentG" will work (normalized to "agentg")
- Old messages queued for "AgentG" won't be found by new queries for "agentg"
- Recommend Option A: accept brief message loss during transition

---

## Testing

```bash
# These should all be equivalent after fix:
curl -X POST /messages -d '{"to": "AgentG", ...}'
curl -X POST /messages -d '{"to": "agentg", ...}'
curl -X POST /messages -d '{"to": "AGENTG", ...}'
curl -X POST /messages -d '{"to": "aGeNtG", ...}'

# All should return same messages:
curl /messages/AgentG
curl /messages/agentg
curl /messages/AGENTG
```

---

## Implementation Checklist

- [ ] Add `normalizeAgentId()` helper to agentmux-server
- [ ] Update `send_message` handler
- [ ] Update `read_messages` handler
- [ ] Update `list_agents` handler
- [ ] Update `inject_reactive` handler
- [ ] Update `get_pending` handler
- [ ] Update `ack_delivery` handler
- [ ] Update MCP tool handlers
- [ ] Update AgentMux poller (defense in depth)
- [ ] Add tests for case variations
- [ ] Deploy and verify

---

## Estimated Effort

- Server changes: 30 minutes
- Testing: 30 minutes
- Deployment: 15 minutes

**Total: ~1-2 hours**
