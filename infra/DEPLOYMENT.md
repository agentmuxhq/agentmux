# WaveMux Webhook Infrastructure - Deployment Guide

Complete guide for deploying the WaveMux webhook router infrastructure.

## Quick Start

```bash
# 1. Navigate to CDK directory
cd infra/cdk

# 2. Install dependencies
npm install

# 3. Configure AWS profile
export AWS_PROFILE=Agent2
export AWS_REGION=us-east-1

# 4. Deploy
npm run build && npm run deploy
```

## Detailed Steps

### 1. Prerequisites

**AWS CLI:**
```bash
aws --version
# Should be 2.x or higher
```

**Node.js:**
```bash
node --version
# Should be 18.x or higher
```

**AWS CDK:**
```bash
npm install -g aws-cdk
cdk --version
# Should be 2.x or higher
```

**AWS Credentials:**
```bash
# Verify Agent2 profile is configured
aws sts get-caller-identity --profile Agent2

# Should show:
# {
#   "UserId": "AIDAQXRFWVRZ2Y5M7SXZY",
#   "Account": "050544946291",
#   "Arn": "arn:aws:iam::050544946291:user/Agent2"
# }
```

### 2. Bootstrap CDK (First Time Only)

```bash
cd infra/cdk

# Bootstrap CDK for your account/region
cdk bootstrap aws://050544946291/us-east-1 --profile Agent2
```

This creates an S3 bucket and other resources needed for CDK deployments.

### 3. Install Dependencies

```bash
cd infra/cdk
npm install
```

### 4. Build and Validate

```bash
# Build TypeScript
npm run build

# Validate CloudFormation template
npm run synth

# View what will be deployed
npm run diff
```

### 5. Deploy to AWS

```bash
# Deploy stack
npm run deploy

# Or with auto-approval (no confirmation prompts)
cdk deploy --require-approval never
```

**Expected output:**
```
✅  wavemux-webhook-prod

Outputs:
wavemux-webhook-prod.HttpApiEndpoint = https://abc123.execute-api.us-east-1.amazonaws.com
wavemux-webhook-prod.WebSocketApiEndpoint = wss://xyz789.execute-api.us-east-1.amazonaws.com/prod
wavemux-webhook-prod.WebhookSecretArn = arn:aws:secretsmanager:us-east-1:050544946291:secret:wavemux/webhook-secret-prod-xxxxx

Stack ARN:
arn:aws:cloudformation:us-east-1:050544946291:stack/wavemux-webhook-prod/...
```

### 6. Configure Secrets

```bash
# Get secret ARN from stack output
SECRET_ARN=$(aws cloudformation describe-stacks \
  --stack-name wavemux-webhook-prod \
  --query 'Stacks[0].Outputs[?OutputKey==`WebhookSecretArn`].OutputValue' \
  --output text \
  --profile Agent2)

# Update secret with your GitHub webhook secret
aws secretsmanager put-secret-value \
  --secret-id "$SECRET_ARN" \
  --secret-string '{
    "github": "your-github-webhook-secret-here",
    "custom": "your-custom-webhook-secret",
    "default": "shared-secret-for-auth"
  }' \
  --profile Agent2
```

**To generate a secure secret:**
```bash
# Generate random secret
openssl rand -hex 32
```

### 7. Test Deployment

```bash
# Get HTTP API endpoint
HTTP_ENDPOINT=$(aws cloudformation describe-stacks \
  --stack-name wavemux-webhook-prod \
  --query 'Stacks[0].Outputs[?OutputKey==`HttpApiEndpoint`].OutputValue' \
  --output text \
  --profile Agent2)

# Test health check
curl "$HTTP_ENDPOINT/health"

# Expected response:
# {
#   "status": "healthy",
#   "service": "wavemux-webhook-router",
#   "environment": "prod",
#   "timestamp": 1730188800
# }
```

### 8. Configure GitHub Webhook

**Get webhook URL:**
```bash
echo "$HTTP_ENDPOINT/webhook/github"
# Example: https://abc123.execute-api.us-east-1.amazonaws.com/webhook/github
```

**In GitHub repository settings:**

1. Go to **Settings** → **Webhooks** → **Add webhook**
2. **Payload URL:** `https://abc123.execute-api.us-east-1.amazonaws.com/webhook/github`
3. **Content type:** `application/json`
4. **Secret:** (use the same secret from Secrets Manager)
5. **Events:** Select events you want (Pull requests, Issues, etc.)
6. **Active:** ✓
7. Click **Add webhook**

### 9. Configure WaveMux Client

Create `~/.wavemux/webhook-config.json`:

```bash
# Get WebSocket endpoint
WS_ENDPOINT=$(aws cloudformation describe-stacks \
  --stack-name wavemux-webhook-prod \
  --query 'Stacks[0].Outputs[?OutputKey==`WebSocketApiEndpoint`].OutputValue' \
  --output text \
  --profile Agent2)

# Generate auth token for your workspace
WORKSPACE_ID="agent2-workspace"
SHARED_SECRET="shared-secret-for-auth"  # Same as in Secrets Manager
AUTH_TOKEN=$(echo -n "$WORKSPACE_ID" | openssl dgst -sha256 -hmac "$SHARED_SECRET" | sed 's/^.* //')

# Create config file
cat > ~/.wavemux/webhook-config.json <<EOF
{
  "version": "1.0",
  "workspaceId": "$WORKSPACE_ID",
  "authToken": "$AUTH_TOKEN",
  "cloudEndpoint": "$WS_ENDPOINT",
  "terminals": []
}
EOF

echo "Config created at ~/.wavemux/webhook-config.json"
```

## Verification

### 1. Verify Stack Resources

```bash
# List all resources in stack
aws cloudformation list-stack-resources \
  --stack-name wavemux-webhook-prod \
  --profile Agent2
```

### 2. Verify DynamoDB Tables

```bash
# List tables
aws dynamodb list-tables --profile Agent2 | grep WaveMux

# Check webhook config table
aws dynamodb describe-table \
  --table-name WaveMuxWebhookConfig-prod \
  --profile Agent2

# Check connections table
aws dynamodb describe-table \
  --table-name WaveMuxConnections-prod \
  --profile Agent2
```

### 3. Verify Lambda Function

```bash
# Get function info
aws lambda get-function \
  --function-name wavemux-webhook-router-prod \
  --profile Agent2

# View recent logs
aws logs tail /aws/lambda/wavemux-webhook-router-prod \
  --follow \
  --profile Agent2
```

### 4. Test Webhook Delivery

```bash
# Create test webhook payload
PAYLOAD='{"action":"opened","pull_request":{"number":123,"title":"Test PR"}}'

# Get GitHub secret from Secrets Manager
GITHUB_SECRET=$(aws secretsmanager get-secret-value \
  --secret-id "$SECRET_ARN" \
  --query 'SecretString' \
  --output text \
  --profile Agent2 | jq -r '.github')

# Calculate signature
SIGNATURE=$(echo -n "$PAYLOAD" | openssl dgst -sha256 -hmac "$GITHUB_SECRET" | sed 's/^.* //')

# Send test webhook
curl -X POST \
  "$HTTP_ENDPOINT/webhook/github" \
  -H "Content-Type: application/json" \
  -H "X-Hub-Signature-256: sha256=$SIGNATURE" \
  -H "X-GitHub-Event: pull_request" \
  -d "$PAYLOAD"

# Expected response:
# {
#   "message": "Webhook processed",
#   "delivered": 0,
#   "total_subscriptions": 0,
#   "errors": null
# }
```

## Monitoring

### CloudWatch Logs

```bash
# Tail Lambda logs
aws logs tail /aws/lambda/wavemux-webhook-router-prod --follow --profile Agent2

# Search for errors
aws logs filter-log-events \
  --log-group-name /aws/lambda/wavemux-webhook-router-prod \
  --filter-pattern "ERROR" \
  --profile Agent2
```

### CloudWatch Metrics

Monitor in AWS Console:
- Lambda → Functions → wavemux-webhook-router-prod → Monitoring
- API Gateway → APIs → wavemux-webhook-http-prod → Dashboard
- DynamoDB → Tables → WaveMuxWebhookConfig-prod → Metrics

### Cost Monitoring

```bash
# Get current month costs
aws ce get-cost-and-usage \
  --time-period Start=$(date -u +%Y-%m-01),End=$(date -u +%Y-%m-%d) \
  --granularity MONTHLY \
  --metrics UnblendedCost \
  --group-by Type=TAG,Key=Project \
  --filter file://<(echo '{
    "Tags": {
      "Key": "Project",
      "Values": ["WaveMux"]
    }
  }') \
  --profile Agent2
```

## Updates and Maintenance

### Update Lambda Code Only

```bash
cd infra/cdk
npm run build
npm run deploy
```

CDK will only update changed resources.

### Update Configuration

```bash
# Update secrets
aws secretsmanager put-secret-value \
  --secret-id "$SECRET_ARN" \
  --secret-string '{"github":"new-secret"}' \
  --profile Agent2
```

### View Drift Detection

```bash
# Check if resources have drifted from template
aws cloudformation detect-stack-drift \
  --stack-name wavemux-webhook-prod \
  --profile Agent2
```

## Rollback

### Rollback to Previous Version

```bash
# List stack events
aws cloudformation describe-stack-events \
  --stack-name wavemux-webhook-prod \
  --profile Agent2

# Rollback if needed
aws cloudformation cancel-update-stack \
  --stack-name wavemux-webhook-prod \
  --profile Agent2
```

### Full Uninstall

```bash
# Delete stack (WARNING: Irreversible!)
npm run destroy

# Or manually:
aws cloudformation delete-stack \
  --stack-name wavemux-webhook-prod \
  --profile Agent2

# Wait for deletion
aws cloudformation wait stack-delete-complete \
  --stack-name wavemux-webhook-prod \
  --profile Agent2
```

## Troubleshooting

### Deployment Fails

**Error: "No default VPC found"**
```bash
# Use explicit VPC (modify stack to accept VPC parameters)
# Or create default VPC:
aws ec2 create-default-vpc --profile Agent2
```

**Error: "Insufficient permissions"**
```bash
# Verify IAM permissions for Agent2
aws iam get-user --profile Agent2
aws iam list-attached-user-policies --user-name Agent2 --profile Agent2
```

### Lambda Invocation Errors

**Check logs:**
```bash
aws logs tail /aws/lambda/wavemux-webhook-router-prod --profile Agent2
```

**Test Lambda directly:**
```bash
aws lambda invoke \
  --function-name wavemux-webhook-router-prod \
  --payload '{"requestContext":{"http":{"method":"GET","path":"/health"}}}' \
  response.json \
  --profile Agent2

cat response.json
```

### WebSocket Connection Issues

**Check connections table:**
```bash
aws dynamodb scan \
  --table-name WaveMuxConnections-prod \
  --profile Agent2
```

**Test WebSocket manually:**
```bash
# Install wscat
npm install -g wscat

# Connect
wscat -c "$WS_ENDPOINT?workspaceId=agent2-workspace&token=$AUTH_TOKEN"
```

## Best Practices

1. **Use separate stacks for dev/prod:**
   - Modify `bin/cdk.ts` to create multiple stacks
   - Deploy with different environment parameters

2. **Enable CloudWatch alarms:**
   - Lambda errors
   - API Gateway 5xx errors
   - DynamoDB throttling

3. **Regular backups:**
   - Enable DynamoDB Point-in-Time Recovery (already enabled for prod)
   - Export webhook configurations periodically

4. **Security:**
   - Rotate webhook secrets regularly
   - Review IAM policies quarterly
   - Enable AWS CloudTrail for audit logging

5. **Cost optimization:**
   - Review CloudWatch logs retention (currently 1 week)
   - Monitor API Gateway request patterns
   - Use DynamoDB on-demand billing

## Next Steps

1. **Implement WaveMux backend integration:**
   - WebSocket client in Go (`pkg/webhookinjector/`)
   - Terminal registration logic
   - Command injection handler

2. **Implement frontend UI:**
   - Webhook configuration modal
   - Subscription management
   - Real-time status indicators

3. **Add monitoring:**
   - CloudWatch dashboards
   - SNS alerts for failures
   - Custom metrics for webhook delivery

4. **Documentation:**
   - User guide for configuring webhooks
   - Examples for common providers
   - Troubleshooting guide

## Support

For issues or questions:
- Check Lambda logs: `aws logs tail /aws/lambda/wavemux-webhook-router-prod --follow`
- Review spec: `SPEC_REACTIVE_AGENT_COMMUNICATION.md`
- GitHub issues: https://github.com/a5af/wavemux/issues
