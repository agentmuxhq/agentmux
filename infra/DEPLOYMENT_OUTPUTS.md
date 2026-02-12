# AgentMux Webhook Infrastructure - Deployment Outputs

Deployed: October 29, 2025 at 10:03 AM EDT

---

## Stack Information

- **Stack Name:** agentmux-webhook-prod
- **Region:** us-east-1
- **Account:** 050544946291
- **Status:** CREATE_COMPLETE
- **Resources Created:** 33/33

---

## API Endpoints

### HTTP API (Webhook Ingestion)
```
https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com
```

**Routes:**
- `POST /webhook/github` - GitHub webhook delivery
- `POST /webhook/custom` - Custom webhook delivery
- `POST /register` - Register webhook subscription
- `POST /unregister` - Remove webhook subscription
- `GET /health` - Health check endpoint

### WebSocket API (Real-time Delivery)
```
wss://oft9nfu83k.execute-api.us-east-1.amazonaws.com/prod
```

**Connection URL Format:**
```
wss://oft9nfu83k.execute-api.us-east-1.amazonaws.com/prod?workspaceId=<workspace>&token=<auth-token>
```

---

## DynamoDB Tables

### Webhook Configuration Table
- **Name:** `AgentMuxWebhookConfig-prod`
- **Purpose:** Store webhook subscriptions and routing rules
- **Primary Key:** `subscriptionId` (String)
- **GSIs:**
  - `ProviderEventIndex` - Query by provider + eventType
  - `WorkspaceIndex` - Query by workspaceId

### Connection Table
- **Name:** `AgentMuxConnections-prod`
- **Purpose:** Track active WebSocket connections
- **Primary Key:** `connectionId` (String)
- **TTL:** 24 hours (automatic cleanup)
- **GSI:** `WorkspaceIndex` - Query by workspaceId

---

## Lambda Function

- **ARN:** `arn:aws:lambda:us-east-1:050544946291:function:agentmux-webhook-router-prod`
- **Name:** `agentmux-webhook-router-prod`
- **Runtime:** Python 3.12
- **Memory:** 512 MB
- **Timeout:** 30 seconds
- **Log Group:** `/aws/lambda/agentmux-webhook-router-prod`

---

## Secrets

- **Name:** `services/prod`
- **Project Key:** `agentmux`
- **Secrets:**
  - `GITHUB_WEBHOOK_SECRET`: 4c9409e245302f804fcab3a849186cde1612a1277a8f130d2e468f25a24197a1
  - `CUSTOM_WEBHOOK_SECRET`: 84b35097110e1e40363d806b82d55ba1f4b384886f76c362510f53dbd84dd0ff
  - `DEFAULT_AUTH_SECRET`: e995488a0ad3349d78c4352abee8b146129d3ed9e66548558fe2a776a4066d19

---

## Next Steps

### 1. Configure GitHub Webhook

1. Go to GitHub repository settings
2. Navigate to **Webhooks** → **Add webhook**
3. Configure:
   - **Payload URL:** `https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com/webhook/github`
   - **Content type:** `application/json`
   - **Secret:** `4c9409e245302f804fcab3a849186cde1612a1277a8f130d2e468f25a24197a1`
   - **Events:** Select desired events (Pull requests, Issues, etc.)
   - **Active:** ✓ Enabled
4. Save webhook

### 2. Test Health Endpoint

```bash
curl https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com/health
```

Expected response:
```json
{
  "status": "healthy",
  "service": "agentmux-webhook-router",
  "environment": "prod",
  "timestamp": "2025-10-29T14:03:00Z"
}
```

### 3. Generate Workspace Auth Token

For AgentMux client authentication:

```bash
# Get the default auth secret
DEFAULT_SECRET="e995488a0ad3349d78c4352abee8b146129d3ed9e66548558fe2a776a4066d19"

# Generate token for a workspace
WORKSPACE_ID="agent2-workspace"
AUTH_TOKEN=$(echo -n "$WORKSPACE_ID" | openssl dgst -sha256 -hmac "$DEFAULT_SECRET" | sed 's/^.* //')

echo "Auth Token for $WORKSPACE_ID: $AUTH_TOKEN"
```

### 4. Update AgentMux Configuration

Create or update `~/.agentmux/webhook-config.json`:

```json
{
  "version": "1.0",
  "workspaceId": "agent2-workspace",
  "authToken": "<generated-token-from-above>",
  "cloudEndpoint": "wss://oft9nfu83k.execute-api.us-east-1.amazonaws.com/prod",
  "terminals": []
}
```

### 5. Register a Subscription

Example: Subscribe to GitHub pull request events

```bash
curl -X POST https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com/register \
  -H "Content-Type: application/json" \
  -d '{
    "workspaceId": "agent2-workspace",
    "terminalId": "terminal-1",
    "provider": "github",
    "eventType": "pull_request",
    "filters": {
      "action": ["opened", "synchronize", "closed"]
    },
    "commandTemplate": "echo \"PR #{{pull_request.number}} {{action}} by {{pull_request.user.login}}\""
  }'
```

---

## Monitoring

### CloudWatch Logs

```bash
# View Lambda logs
aws logs tail /aws/lambda/agentmux-webhook-router-prod --follow --profile Agent2
```

### DynamoDB Console

- [WebhookConfigTable](https://console.aws.amazon.com/dynamodbv2/home?region=us-east-1#table?name=AgentMuxWebhookConfig-prod)
- [ConnectionTable](https://console.aws.amazon.com/dynamodbv2/home?region=us-east-1#table?name=AgentMuxConnections-prod)

### Lambda Console

- [WebhookRouterFunction](https://console.aws.amazon.com/lambda/home?region=us-east-1#/functions/agentmux-webhook-router-prod)

---

## Cost Estimate

Based on expected usage (100 webhooks/day, 5 active terminals):

| Service | Monthly Cost |
|---------|--------------|
| API Gateway HTTP | $3.00 |
| API Gateway WebSocket | $0.25 |
| Lambda | $0.20 (free tier) |
| DynamoDB | $1.25 |
| Secrets Manager | $0.40 |
| CloudWatch Logs | $0.05 |
| Data Transfer | $0.20 |
| **Total** | **~$5.35/month** |

---

## Stack Outputs (CDK)

Export names for cross-stack references:

- `agentmux-webhook-prod-WebhookConfigTable`: AgentMuxWebhookConfig-prod
- `agentmux-webhook-prod-ConnectionTable`: AgentMuxConnections-prod
- `agentmux-webhook-prod-WebhookRouterArn`: arn:aws:lambda:us-east-1:050544946291:function:agentmux-webhook-router-prod
- `agentmux-webhook-prod-HttpApiEndpoint`: https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com
- `agentmux-webhook-prod-WebSocketApiEndpoint`: wss://oft9nfu83k.execute-api.us-east-1.amazonaws.com/prod
- `agentmux-webhook-prod-SecretName`: services/prod

---

## Cleanup

To delete the entire stack:

```bash
cd /d/Code/agent-workspaces/agent2/agentmux/infra/cdk
cdk destroy --profile Agent2
```

**Note:** DynamoDB tables with `RETAIN` policy (WebhookConfigTable in prod) will not be deleted automatically.
