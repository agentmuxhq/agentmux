# Add AgentMux secrets to services/prod
# PowerShell script for Windows

$GITHUB_SECRET = "4c9409e245302f804fcab3a849186cde1612a1277a8f130d2e468f25a24197a1"
$CUSTOM_SECRET = "84b35097110e1e40363d806b82d55ba1f4b384886f76c362510f53dbd84dd0ff"
$DEFAULT_SECRET = "e995488a0ad3349d78c4352abee8b146129d3ed9e66548558fe2a776a4066d19"

Write-Host "Fetching current services/prod secret..."
$currentSecretJson = aws secretsmanager get-secret-value `
    --secret-id services/prod `
    --profile Agent2 `
    --region us-east-1 `
    --query SecretString `
    --output text

# Parse the current secret
$currentSecret = $currentSecretJson | ConvertFrom-Json

# Add agentmux key
$agentmuxSecret = @{
    GITHUB_WEBHOOK_SECRET = $GITHUB_SECRET
    CUSTOM_WEBHOOK_SECRET = $CUSTOM_SECRET
    DEFAULT_AUTH_SECRET = $DEFAULT_SECRET
}

$currentSecret | Add-Member -NotePropertyName "agentmux" -NotePropertyValue $agentmuxSecret -Force

# Convert back to JSON
$updatedSecretJson = $currentSecret | ConvertTo-Json -Depth 10 -Compress

# Save to temp file
$tempFile = [System.IO.Path]::GetTempFileName()
$updatedSecretJson | Out-File -FilePath $tempFile -Encoding utf8 -NoNewline

Write-Host "Updating services/prod in AWS Secrets Manager..."
aws secretsmanager put-secret-value `
    --secret-id services/prod `
    --profile Agent2 `
    --region us-east-1 `
    --secret-string "file://$tempFile"

Remove-Item $tempFile

Write-Host "`nDone! Verifying..."
$verifyJson = aws secretsmanager get-secret-value `
    --secret-id services/prod `
    --profile Agent2 `
    --region us-east-1 `
    --query SecretString `
    --output text

$verifySecret = $verifyJson | ConvertFrom-Json
Write-Host "`nAgentMux secrets:"
$verifySecret.agentmux | ConvertTo-Json -Depth 3
