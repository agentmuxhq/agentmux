#!/bin/bash
# Add WaveMux secrets to services/prod

set -e

GITHUB_SECRET="4c9409e245302f804fcab3a849186cde1612a1277a8f130d2e468f25a24197a1"
CUSTOM_SECRET="84b35097110e1e40363d806b82d55ba1f4b384886f76c362510f53dbd84dd0ff"
DEFAULT_SECRET="e995488a0ad3349d78c4352abee8b146129d3ed9e66548558fe2a776a4066d19"

echo "Fetching current services/prod secret..."
CURRENT=$(aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --region us-east-1 \
  --query SecretString \
  --output text)

echo "Adding wavemux secrets..."
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

echo "Updating services/prod in AWS Secrets Manager..."
echo "$UPDATED" | aws secretsmanager put-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --region us-east-1 \
  --secret-string file:///dev/stdin

echo "Done! Verifying..."
aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --region us-east-1 \
  --query SecretString \
  --output text | jq '.wavemux'
