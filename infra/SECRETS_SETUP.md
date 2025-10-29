# WaveMux Webhook Secrets Setup

Guide for configuring webhook secrets in AWS Secrets Manager using the standard `services/prod` encoding scheme.

---

## Secret Structure

All WaveMux webhook secrets are stored in the existing **`services/prod`** secret under the **`wavemux`** project key.

**Path:** `services/prod["wavemux"]`

---

## Required Secrets

The following secrets need to be added to the `wavemux` key:

| Secret Name | Purpose | Example Value |
|------------|---------|---------------|
| `GITHUB_WEBHOOK_SECRET` | GitHub webhook signature validation | `your-github-webhook-secret` |
| `CUSTOM_WEBHOOK_SECRET` | Custom webhook authentication | `your-custom-secret` |
| `DEFAULT_AUTH_SECRET` | WebSocket client authentication | `shared-secret-for-workspace-auth` |

---

## Setup Instructions

### 1. Fetch Current Secret

```bash
# Fetch current services/prod secret
CURRENT=$(aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text)

# View current structure
echo "$CURRENT" | jq .
```

### 2. Add WaveMux Secrets

```bash
# Generate secure secrets (optional)
GITHUB_SECRET=$(openssl rand -hex 32)
CUSTOM_SECRET=$(openssl rand -hex 32)
DEFAULT_SECRET=$(openssl rand -hex 32)

# Add wavemux secrets to the structure
UPDATED=$(echo "$CURRENT" | jq --arg github "$GITHUB_SECRET" \
  --arg custom "$CUSTOM_SECRET" \
  --arg default "$DEFAULT_SECRET" \
  '. + {
    "wavemux": {
      "GITHUB_WEBHOOK_SECRET": $github,
      "CUSTOM_WEBHOOK_SECRET": $custom,
      "DEFAULT_AUTH_SECRET": $default
    }
  }')

# Write back to AWS Secrets Manager
echo "$UPDATED" | aws secretsmanager put-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --secret-string file:///dev/stdin
```

### 3. Verify Secrets

```bash
# View wavemux secrets
aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text | jq '.wavemux'
```

**Expected output:**
```json
{
  "GITHUB_WEBHOOK_SECRET": "abc123...",
  "CUSTOM_WEBHOOK_SECRET": "def456...",
  "DEFAULT_AUTH_SECRET": "ghi789..."
}
```

---

## Final Structure

After setup, `services/prod` will have this structure:

```json
{
  "pulse": { ... },
  "askbase": { ... },
  "stratum": { ... },
  "wavemux": {
    "GITHUB_WEBHOOK_SECRET": "your-github-webhook-secret",
    "CUSTOM_WEBHOOK_SECRET": "your-custom-secret",
    "DEFAULT_AUTH_SECRET": "shared-secret-for-workspace-auth"
  },
  ...other projects...
}
```

---

## GitHub Webhook Configuration

### 1. Get Webhook URL

After CDK deployment:

```bash
# Get HTTP API endpoint
HTTP_ENDPOINT=$(aws cloudformation describe-stacks \
  --stack-name wavemux-webhook-prod \
  --profile Agent2 \
  --query 'Stacks[0].Outputs[?OutputKey==`HttpApiEndpoint`].OutputValue' \
  --output text)

echo "GitHub Webhook URL: $HTTP_ENDPOINT/webhook/github"
```

### 2. Configure in GitHub

1. Go to repository **Settings** → **Webhooks** → **Add webhook**
2. **Payload URL:** `https://{api-id}.execute-api.us-east-1.amazonaws.com/webhook/github`
3. **Content type:** `application/json`
4. **Secret:** Use the value of `GITHUB_WEBHOOK_SECRET` from Secrets Manager
5. **Events:** Select desired events (Pull requests, Issues, etc.)
6. **Active:** ✓ Enabled
7. Click **Add webhook**

### 3. Get GitHub Secret Value

```bash
# Retrieve GitHub webhook secret for configuration
aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text | jq -r '.wavemux.GITHUB_WEBHOOK_SECRET'
```

Copy this value to GitHub webhook configuration.

---

## WaveMux Client Authentication

### Generate Workspace Auth Token

Each WaveMux workspace needs an authentication token to connect via WebSocket.

```bash
# Get the default auth secret
DEFAULT_SECRET=$(aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text | jq -r '.wavemux.DEFAULT_AUTH_SECRET')

# Generate token for a workspace
WORKSPACE_ID="agent2-workspace"
AUTH_TOKEN=$(echo -n "$WORKSPACE_ID" | openssl dgst -sha256 -hmac "$DEFAULT_SECRET" | sed 's/^.* //')

echo "Auth Token for $WORKSPACE_ID: $AUTH_TOKEN"
```

### Add to WaveMux Config

Update `~/.wavemux/webhook-config.json`:

```json
{
  "version": "1.0",
  "workspaceId": "agent2-workspace",
  "authToken": "generated-token-from-above",
  "cloudEndpoint": "wss://{api-id}.execute-api.us-east-1.amazonaws.com/prod",
  "terminals": []
}
```

---

## Testing Secrets

### Test GitHub Webhook Secret

```bash
# Get secret
GITHUB_SECRET=$(aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text | jq -r '.wavemux.GITHUB_WEBHOOK_SECRET')

# Create test payload
PAYLOAD='{"action":"opened","pull_request":{"number":123}}'

# Calculate signature
SIGNATURE=$(echo -n "$PAYLOAD" | openssl dgst -sha256 -hmac "$GITHUB_SECRET" | sed 's/^.* //')

# Get HTTP endpoint
HTTP_ENDPOINT=$(aws cloudformation describe-stacks \
  --stack-name wavemux-webhook-prod \
  --profile Agent2 \
  --query 'Stacks[0].Outputs[?OutputKey==`HttpApiEndpoint`].OutputValue' \
  --output text)

# Send test webhook
curl -X POST "$HTTP_ENDPOINT/webhook/github" \
  -H "Content-Type: application/json" \
  -H "X-Hub-Signature-256: sha256=$SIGNATURE" \
  -H "X-GitHub-Event: pull_request" \
  -d "$PAYLOAD"
```

### Test WebSocket Authentication

```bash
# Get WebSocket endpoint
WS_ENDPOINT=$(aws cloudformation describe-stacks \
  --stack-name wavemux-webhook-prod \
  --profile Agent2 \
  --query 'Stacks[0].Outputs[?OutputKey==`WebSocketApiEndpoint`].OutputValue' \
  --output text)

# Get auth token (calculated above)
# Test connection with wscat
npm install -g wscat
wscat -c "$WS_ENDPOINT?workspaceId=agent2-workspace&token=$AUTH_TOKEN"
```

---

## Secret Rotation

### Rotate GitHub Webhook Secret

```bash
# Generate new secret
NEW_GITHUB_SECRET=$(openssl rand -hex 32)

# Update in AWS
CURRENT=$(aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text)

UPDATED=$(echo "$CURRENT" | jq \
  --arg secret "$NEW_GITHUB_SECRET" \
  '.wavemux.GITHUB_WEBHOOK_SECRET = $secret')

echo "$UPDATED" | aws secretsmanager put-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --secret-string file:///dev/stdin

# Update in GitHub webhook settings
echo "New GitHub webhook secret: $NEW_GITHUB_SECRET"
```

### Rotate WebSocket Auth Secret

```bash
# Generate new secret
NEW_DEFAULT_SECRET=$(openssl rand -hex 32)

# Update in AWS
CURRENT=$(aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text)

UPDATED=$(echo "$CURRENT" | jq \
  --arg secret "$NEW_DEFAULT_SECRET" \
  '.wavemux.DEFAULT_AUTH_SECRET = $secret')

echo "$UPDATED" | aws secretsmanager put-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --secret-string file:///dev/stdin

# Regenerate all workspace tokens with new secret
```

---

## Troubleshooting

### Secret Not Found

**Error:** `KeyError: 'wavemux'` or secrets not loading

**Solution:**
```bash
# Check if wavemux key exists
aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text | jq 'has("wavemux")'

# If false, add the wavemux key (see Setup Instructions)
```

### Invalid Signature

**Error:** GitHub webhook delivery fails with 401

**Possible causes:**
1. Secret mismatch between AWS and GitHub
2. Signature calculation error

**Solution:**
```bash
# Verify secret in AWS matches GitHub
aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text | jq -r '.wavemux.GITHUB_WEBHOOK_SECRET'

# Update GitHub webhook settings with correct secret
```

### WebSocket Connection Refused

**Error:** Connection fails with 401

**Possible causes:**
1. Invalid auth token
2. Workspace ID mismatch

**Solution:**
```bash
# Regenerate auth token
DEFAULT_SECRET=$(aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text | jq -r '.wavemux.DEFAULT_AUTH_SECRET')

WORKSPACE_ID="agent2-workspace"
AUTH_TOKEN=$(echo -n "$WORKSPACE_ID" | openssl dgst -sha256 -hmac "$DEFAULT_SECRET" | sed 's/^.* //')

# Update ~/.wavemux/webhook-config.json with new token
```

---

## Security Best Practices

1. **Generate Strong Secrets**
   ```bash
   openssl rand -hex 32  # 256-bit entropy
   ```

2. **Rotate Regularly**
   - Rotate webhook secrets every 90 days
   - Rotate after any security incident

3. **Limit Access**
   - Only grant `secretsmanager:GetSecretValue` to Lambda execution role
   - Use IAM conditions to restrict access to `services/prod`

4. **Monitor Usage**
   ```bash
   # View secret access logs in CloudTrail
   aws cloudtrail lookup-events \
     --lookup-attributes AttributeKey=ResourceName,AttributeValue=services/prod \
     --profile Agent2
   ```

5. **Audit Periodically**
   - Review which projects have access
   - Check for unused secrets
   - Verify secret values haven't been exposed

---

## Reference

- **Secret Name:** `services/prod`
- **Project Key:** `wavemux`
- **Lambda Environment Variables:**
  - `SECRET_NAME=services/prod`
  - `PROJECT_NAME=wavemux`

**Related Documentation:**
- [DEPLOYMENT.md](DEPLOYMENT.md) - Infrastructure deployment
- [AWS_RESOURCES.md](AWS_RESOURCES.md) - Resource inventory
- [Shared Infrastructure](../../shared-infrastructure/README.md) - Services secret pattern
