# AWS Resources Created by CDK

Complete list of AWS resources that will be created when deploying the WaveMux Webhook Infrastructure.

---

## Summary

**Total Resources:** 33 AWS resources
**Environment:** prod (configurable)

---

## Resource Breakdown by Service

### 1. DynamoDB Tables (2)

| Resource Type | Logical ID | Physical Name | Purpose |
|--------------|------------|---------------|---------|
| `AWS::DynamoDB::Table` | `WebhookConfigTable0E03631F` | `WaveMuxWebhookConfig-prod` | Stores webhook subscriptions and routing configuration |
| `AWS::DynamoDB::Table` | `ConnectionTable0C6E1E44` | `WaveMuxConnections-prod` | Tracks active WebSocket connections |

**Features:**
- PAY_PER_REQUEST billing mode (no provisioned capacity)
- Global Secondary Indexes (GSIs) for efficient queries
- Point-in-Time Recovery (PITR) enabled for prod
- TTL enabled on ConnectionTable for automatic cleanup
- Encryption at rest (AWS managed keys)

**Table Details:**

**WebhookConfigTable:**
- Primary Key: `subscriptionId` (String)
- GSI 1: `ProviderEventIndex` - Query by provider + event type
- GSI 2: `WorkspaceIndex` - Query by workspace ID

**ConnectionTable:**
- Primary Key: `connectionId` (String)
- GSI: `WorkspaceIndex` - Query by workspace ID
- TTL: `ttl` attribute (24 hour expiration)

---

### 2. Lambda Functions (2)

| Resource Type | Logical ID | Physical Name | Runtime |
|--------------|------------|---------------|---------|
| `AWS::Lambda::Function` | `WebhookRouterFunctionB37723DF` | `wavemux-webhook-router-prod` | Python 3.12 |
| `AWS::Lambda::Function` | `LogRetentionaae0aa3c5b4d4f87b02d85b201efdd8aFD4BFC8A` | (CDK internal) | Node.js 18.x |

**Main Function (WebhookRouter):**
- Memory: 512 MB
- Timeout: 30 seconds
- Handler: `handler.lambda_handler`
- Environment Variables:
  - `WEBHOOK_CONFIG_TABLE`
  - `CONNECTION_TABLE`
  - `ENVIRONMENT`
  - `WEBHOOK_SECRET_ARN`

**Log Retention Function:**
- Internal CDK function to manage CloudWatch log retention
- Automatically created by CDK

---

### 3. API Gateway v2 (2 APIs, 14 routes/integrations)

#### HTTP API (for webhooks)

| Resource Type | Logical ID | Purpose |
|--------------|------------|---------|
| `AWS::ApiGatewayV2::Api` | `HttpApiF5A9A8A7` | Main HTTP API |
| `AWS::ApiGatewayV2::Stage` | `HttpApiDefaultStage3EEB07D6` | Default stage |

**Routes (4):**
1. `POST /webhook/{provider}` - Receive webhooks
2. `POST /register` - Register subscriptions
3. `POST /unregister` - Remove subscriptions
4. `GET /health` - Health check

**Integrations (4):**
- All routes integrate with `WebhookRouterFunction`
- Type: AWS_PROXY (Lambda proxy integration)

**Permissions (4):**
- Lambda permissions for HTTP API to invoke function

#### WebSocket API (for WaveMux clients)

| Resource Type | Logical ID | Purpose |
|--------------|------------|---------|
| `AWS::ApiGatewayV2::Api` | `WebSocketApi34BCF99B` | Main WebSocket API |
| `AWS::ApiGatewayV2::Stage` | `WebSocketStageC46B7E43` | Stage (prod) |

**Routes (2):**
1. `$connect` - Client connection
2. `$disconnect` - Client disconnection

**Integrations (2):**
- Both routes integrate with `WebhookRouterFunction`

**Permissions (2):**
- Lambda permissions for WebSocket API to invoke function

---

### 4. IAM Roles & Policies (4)

| Resource Type | Logical ID | Attached To |
|--------------|------------|-------------|
| `AWS::IAM::Role` | `WebhookRouterFunctionServiceRole0D49BDE5` | WebhookRouterFunction |
| `AWS::IAM::Policy` | `WebhookRouterFunctionServiceRoleDefaultPolicy626A7F06` | WebhookRouterFunction Role |
| `AWS::IAM::Role` | `LogRetentionaae0aa3c5b4d4f87b02d85b201efdd8aServiceRole9741ECFB` | Log Retention Function |
| `AWS::IAM::Policy` | `LogRetentionaae0aa3c5b4d4f87b02d85b201efdd8aServiceRoleDefaultPolicyADDA7DEB` | Log Retention Role |

**WebhookRouterFunction Permissions:**
- DynamoDB: Read/Write to both tables
- Secrets Manager: Read webhook secrets
- API Gateway: ManageConnections (for WebSocket)
- CloudWatch Logs: Create log streams and put events

---

### 5. Secrets Manager (1)

| Resource Type | Logical ID | Physical Name |
|--------------|------------|---------------|
| `AWS::SecretsManager::Secret` | `WebhookSecretF21E29CC` | `wavemux/webhook-secret-prod` |

**Purpose:** Store webhook authentication secrets
**Format:**
```json
{
  "github": "your-github-webhook-secret",
  "custom": "your-custom-secret",
  "default": "shared-secret-for-auth"
}
```

---

### 6. CloudWatch Logs (1 Custom Resource)

| Resource Type | Logical ID | Purpose |
|--------------|------------|---------|
| `Custom::LogRetention` | `WebhookRouterFunctionLogRetention3E42EC11` | Manage log retention |

**Configuration:**
- Log Group: `/aws/lambda/wavemux-webhook-router-prod`
- Retention: 7 days

---

### 7. CDK Metadata (1)

| Resource Type | Logical ID | Purpose |
|--------------|------------|---------|
| `AWS::CDK::Metadata` | `CDKMetadata` | CDK version tracking |

---

## Complete Resource List (33 total)

### By Service:
- **DynamoDB:** 2 tables
- **Lambda:** 2 functions, 6 permissions
- **API Gateway v2:** 2 APIs, 2 stages, 6 routes, 6 integrations
- **IAM:** 2 roles, 2 policies
- **Secrets Manager:** 1 secret
- **CloudWatch Logs:** 1 custom resource
- **CDK:** 1 metadata resource

### Logical IDs:
1. `ConnectionTable0C6E1E44`
2. `WebhookConfigTable0E03631F`
3. `WebhookRouterFunctionB37723DF`
4. `LogRetentionaae0aa3c5b4d4f87b02d85b201efdd8aFD4BFC8A`
5. `WebhookRouterFunctionServiceRole0D49BDE5`
6. `WebhookRouterFunctionServiceRoleDefaultPolicy626A7F06`
7. `LogRetentionaae0aa3c5b4d4f87b02d85b201efdd8aServiceRole9741ECFB`
8. `LogRetentionaae0aa3c5b4d4f87b02d85b201efdd8aServiceRoleDefaultPolicyADDA7DEB`
9. `HttpApiF5A9A8A7`
10. `HttpApiDefaultStage3EEB07D6`
11. `HttpApiGEThealthAB1F4856`
12. `HttpApiPOSTregister65F19B6F`
13. `HttpApiPOSTunregister91C3751D`
14. `HttpApiPOSTwebhookprovider24AB8CD1`
15. `HttpApiGEThealthHealthIntegration700E4B25`
16. `HttpApiPOSTregisterRegisterIntegration1A4ACC38`
17. `HttpApiPOSTunregisterUnregisterIntegration2074F446`
18. `HttpApiPOSTwebhookproviderWebhookDeliveryIntegration6F0571D4`
19. `HttpApiGEThealthHealthIntegrationPermissionEC5DEC43`
20. `HttpApiPOSTregisterRegisterIntegrationPermissionD3BA9386`
21. `HttpApiPOSTunregisterUnregisterIntegrationPermission8CE9FB0F`
22. `HttpApiPOSTwebhookproviderWebhookDeliveryIntegrationPermission80433BE6`
23. `WebSocketApi34BCF99B`
24. `WebSocketStageC46B7E43`
25. `WebSocketApiconnectRoute846149DD`
26. `WebSocketApidisconnectRouteC181A19C`
27. `WebSocketApiconnectRouteConnectIntegration7F1E0FDE`
28. `WebSocketApidisconnectRouteDisconnectIntegration94C91381`
29. `WebSocketApiconnectRouteConnectIntegrationPermission39398969`
30. `WebSocketApidisconnectRouteDisconnectIntegrationPermissionAE705904`
31. `WebhookSecretF21E29CC`
32. `WebhookRouterFunctionLogRetention3E42EC11`
33. `CDKMetadata`

---

## Resource Naming Convention

**Physical Names:**
- DynamoDB Tables: `WaveMux{TableName}-{env}`
- Lambda Functions: `wavemux-webhook-router-{env}`
- Secrets: `wavemux/webhook-secret-{env}`
- API Gateway: Auto-generated IDs
- IAM Roles/Policies: Auto-generated by CDK

**Logical IDs:**
- Generated by CDK based on construct IDs
- Include random suffixes for uniqueness

---

## Stack Outputs (6 Exports)

After deployment, the following outputs are available:

| Output Name | Export Name | Value |
|------------|-------------|-------|
| `WebhookConfigTableName` | `wavemux-webhook-prod-WebhookConfigTable` | Table name |
| `ConnectionTableName` | `wavemux-webhook-prod-ConnectionTable` | Table name |
| `WebhookRouterFunctionArn` | `wavemux-webhook-prod-WebhookRouterArn` | Lambda ARN |
| `HttpApiEndpoint` | `wavemux-webhook-prod-HttpApiEndpoint` | HTTP API URL |
| `WebSocketApiEndpoint` | `wavemux-webhook-prod-WebSocketApiEndpoint` | WebSocket URL |
| `WebhookSecretArn` | `wavemux-webhook-prod-WebhookSecretArn` | Secret ARN |

---

## Deployment Commands

### View Resources Before Deploy
```bash
cd infra/cdk
cdk synth
```

### List All Resources
```bash
cdk synth | grep "Type:"
```

### Deploy Stack
```bash
cdk deploy --profile Agent2
```

### View Deployed Resources
```bash
aws cloudformation describe-stack-resources \
  --stack-name wavemux-webhook-prod \
  --profile Agent2
```

---

## Cost Estimate

**Monthly Costs (100 webhooks/day, 5 active terminals):**

| Service | Usage | Cost |
|---------|-------|------|
| **API Gateway HTTP** | 3,000 requests/month | $3.00 |
| **API Gateway WebSocket** | 5 connections × 24h × 30d | $0.25 |
| **Lambda** | 3,000 invocations, 512MB, 1s avg | $0.20 (free tier) |
| **DynamoDB** | ~10,000 R/W requests | $1.25 |
| **Secrets Manager** | 1 secret, ~100 retrievals | $0.40 |
| **CloudWatch Logs** | ~100 MB/month | $0.05 |
| **Data Transfer** | Minimal | $0.20 |
| **Total** | | **~$5.35/month** |

**Free Tier Benefits:**
- Lambda: First 1M requests/month free
- DynamoDB: 25GB storage free
- CloudWatch Logs: 5GB ingestion free

---

## Resource Dependencies

```
WebhookConfigTable ─┐
ConnectionTable ────┼──> WebhookRouterFunction ──> Lambda Role
WebhookSecret ──────┘           │                      │
                                │                      └──> IAM Policy
                                ├──> HTTP API
                                └──> WebSocket API
```

---

## Cleanup (Delete Stack)

To delete all resources:

```bash
# Delete stack
cdk destroy --profile Agent2

# Verify deletion
aws cloudformation describe-stacks \
  --stack-name wavemux-webhook-prod \
  --profile Agent2
```

**Note:** DynamoDB tables with `RETAIN` policy in prod will not be deleted automatically.

---

## Tags Applied to All Resources

| Tag Key | Tag Value |
|---------|-----------|
| `Project` | `WaveMux` |
| `Component` | `WebhookRouter` |
| `Environment` | `prod` (or configured value) |

---

**Generated From:** `cdk synth` output
**Stack Name:** `wavemux-webhook-prod`
**Region:** `us-east-1`
**CDK Version:** 2.1030.0
