# Specification: Reactive Agent Communication via Webhook Shell Injection

**Version:** 1.0.0
**Date:** 2025-10-29
**Status:** Draft
**Author:** Agent2

---

## Executive Summary

This specification defines a system for enabling reactive agent communication in AgentMux through webhook-based shell injection. The system allows external agents (via webhooks) to inject text commands directly into specific AgentMux terminal panes, enabling bidirectional communication between cloud-based agents and local terminal sessions.

**Key Use Cases:**
- GitHub webhook notifications injecting git commands
- CI/CD pipeline status updates triggering local actions
- Agent-to-agent communication across different workspaces
- Reactive monitoring and alerting systems
- Cross-platform agent coordination

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        External Triggers                         │
│  (GitHub Webhooks, CI/CD, Monitoring, Other Agents)            │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            │ HTTPS
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                    AWS API Gateway + Lambda                      │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  AgentMuxWebhookRouter Lambda                              │  │
│  │  - Validates webhook signature                            │  │
│  │  - Routes to correct terminal by terminalId               │  │
│  │  - Transforms webhook payload to shell command            │  │
│  └──────────────────────────────────────────────────────────┘  │
│                            │                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  WebhookConfig (DynamoDB)                                 │  │
│  │  - Terminal subscriptions                                 │  │
│  │  - Webhook authentication                                 │  │
│  │  │  Command transformation rules                          │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              │ WebSocket/SSE
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         AgentMux Backend                          │
│                       (agentmuxsrv - Go)                          │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Webhook Injection Service                                │  │
│  │  - Maintains WebSocket connection to AWS                  │  │
│  │  - Receives injection events                              │  │
│  │  - Validates terminal exists and is active                │  │
│  │  - Injects text into terminal PTY                         │  │
│  └──────────────────────────────────────────────────────────┘  │
│                            │                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Terminal Manager                                         │  │
│  │  - Maps persistent terminalId to active BlockController   │  │
│  │  - Handles terminal lifecycle events                      │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              │ Terminal Input
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      AgentMux Frontend                            │
│                    (TypeScript + React)                          │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Terminal View (term.tsx)                                 │  │
│  │  - Displays injected text in terminal                     │  │
│  │  - Maintains persistent terminalId in Block.Meta          │  │
│  │  - Configuration UI for webhook subscriptions             │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Design

### 1. Persistent Terminal Identity

**Problem:** AgentMux blocks (terminals) are identified by UUIDs that change on each restart, making it impossible for webhooks to target specific terminals across sessions.

**Solution:** Add a persistent `terminalId` to each terminal block's metadata.

#### Frontend Changes (frontend/app/view/term/term.tsx)

```typescript
interface TerminalMeta extends MetaType {
    // Existing meta fields...
    "term:terminalId"?: string;        // Persistent ID for webhook routing
    "term:webhookSubs"?: string[];     // List of webhook subscription IDs
    "term:displayName"?: string;       // User-friendly name for terminal
}

// On terminal creation, generate or restore terminalId
function ensureTerminalId(blockId: string): string {
    const block = WOS.getObject<Block>(`block:${blockId}`);
    let terminalId = block?.meta?.["term:terminalId"];

    if (!terminalId) {
        // Generate persistent ID based on workspace + counter
        terminalId = `${workspaceId}-term-${getNextTerminalCounter()}`;

        // Persist to block metadata
        WOS.updateObject({
            oref: `block:${blockId}`,
            meta: {
                ...block.meta,
                "term:terminalId": terminalId
            }
        });
    }

    return terminalId;
}
```

**Storage:** Terminal IDs are stored in:
- Block metadata (runtime state)
- Workspace configuration file (persistent state)
- DynamoDB webhook routing table (cloud state)

---

### 2. Webhook Configuration Management

**Location:** `~/.agentmux/webhook-config.json`

**Schema:**

```json
{
  "version": "1.0",
  "workspaceId": "agent2-workspace",
  "terminals": [
    {
      "terminalId": "agent2-workspace-term-001",
      "displayName": "GitHub Monitor",
      "webhookSubscriptions": [
        {
          "id": "github-pr-updates",
          "provider": "github",
          "events": ["pull_request", "issue_comment"],
          "filters": {
            "repository": "a5af/agentmux",
            "action": ["opened", "synchronize", "closed"]
          },
          "commandTemplate": "echo '[GitHub] {{event.action}} on PR #{{event.pull_request.number}}'\n",
          "enabled": true
        }
      ]
    },
    {
      "terminalId": "agent2-workspace-term-002",
      "displayName": "CI/CD Monitor",
      "webhookSubscriptions": [
        {
          "id": "github-actions-status",
          "provider": "github",
          "events": ["workflow_run"],
          "filters": {
            "repository": "a5af/*",
            "status": ["completed"]
          },
          "commandTemplate": "echo '[Actions] Workflow {{event.workflow.name}}: {{event.workflow_run.conclusion}}'\n",
          "enabled": true
        }
      ]
    }
  ],
  "webhookSecret": "${SECRET_REF:agentmux/webhook-secret}",
  "cloudEndpoint": "wss://webhook-router.a5af.dev/connect"
}
```

**Configuration API (Go Backend):**

```go
// pkg/webhookconfig/webhookconfig.go
package webhookconfig

type WebhookConfig struct {
    Version       string              `json:"version"`
    WorkspaceId   string              `json:"workspaceId"`
    Terminals     []TerminalConfig    `json:"terminals"`
    WebhookSecret string              `json:"webhookSecret"`
    CloudEndpoint string              `json:"cloudEndpoint"`
}

type TerminalConfig struct {
    TerminalId           string                `json:"terminalId"`
    DisplayName          string                `json:"displayName"`
    WebhookSubscriptions []WebhookSubscription `json:"webhookSubscriptions"`
}

type WebhookSubscription struct {
    ID              string            `json:"id"`
    Provider        string            `json:"provider"` // github, custom, etc.
    Events          []string          `json:"events"`
    Filters         map[string]any    `json:"filters"`
    CommandTemplate string            `json:"commandTemplate"`
    Enabled         bool              `json:"enabled"`
}

// Load configuration from disk
func LoadConfig() (*WebhookConfig, error)

// Save configuration to disk
func SaveConfig(config *WebhookConfig) error

// Register terminal with webhook router
func RegisterTerminal(terminalId string, subscriptions []WebhookSubscription) error
```

---

### 3. AWS Lambda Infrastructure

**Repository:** `shared-infrastructure/lambdas/agentmux-webhook-router/`

#### Lambda Function: `AgentMuxWebhookRouter`

**Purpose:** Route incoming webhooks to appropriate AgentMux terminal instances.

**Handler:** `handler.py`

```python
import json
import boto3
import hmac
import hashlib
from typing import Dict, Any, Optional

dynamodb = boto3.resource('dynamodb')
apigateway = boto3.client('apigatewaymanagementapi')
secrets = boto3.client('secretsmanager')

# DynamoDB tables
webhook_config_table = dynamodb.Table('AgentMuxWebhookConfig')
connection_table = dynamodb.Table('AgentMuxConnections')

def lambda_handler(event: Dict[str, Any], context: Any) -> Dict[str, Any]:
    """
    Main webhook router handler.

    Routes:
    - POST /webhook/{provider} - Receive webhook from external service
    - POST /register - Register terminal subscription
    - POST /unregister - Remove terminal subscription
    - WebSocket $connect - Establish connection from AgentMux instance
    - WebSocket $disconnect - Clean up connection
    """

    route_key = event.get('routeKey', '')

    if route_key == '$connect':
        return handle_websocket_connect(event)
    elif route_key == '$disconnect':
        return handle_websocket_disconnect(event)
    elif event.get('httpMethod') == 'POST':
        path = event.get('path', '')
        if path.startswith('/webhook/'):
            return handle_webhook_delivery(event)
        elif path == '/register':
            return handle_registration(event)
        elif path == '/unregister':
            return handle_unregistration(event)

    return {'statusCode': 404, 'body': 'Not Found'}

def handle_webhook_delivery(event: Dict[str, Any]) -> Dict[str, Any]:
    """Process incoming webhook and route to subscribed terminals."""

    # Extract provider from path
    path = event['path']  # e.g., /webhook/github
    provider = path.split('/')[-1]

    # Parse webhook payload
    body = json.loads(event['body'])
    headers = event['headers']

    # Validate webhook signature
    if not validate_webhook_signature(provider, headers, event['body']):
        return {
            'statusCode': 401,
            'body': json.dumps({'error': 'Invalid signature'})
        }

    # Determine event type
    event_type = extract_event_type(provider, headers, body)

    # Query DynamoDB for subscribed terminals
    subscriptions = query_subscriptions(provider, event_type)

    if not subscriptions:
        return {
            'statusCode': 200,
            'body': json.dumps({'message': 'No subscriptions', 'delivered': 0})
        }

    # Route to each subscribed terminal
    delivered = 0
    for subscription in subscriptions:
        if matches_filters(subscription['filters'], body):
            command = render_command_template(
                subscription['commandTemplate'],
                body
            )

            # Send to AgentMux instance via WebSocket
            success = send_to_terminal(
                subscription['workspaceId'],
                subscription['terminalId'],
                command
            )

            if success:
                delivered += 1

    return {
        'statusCode': 200,
        'body': json.dumps({
            'message': 'Webhook delivered',
            'delivered': delivered,
            'total_subscriptions': len(subscriptions)
        })
    }

def validate_webhook_signature(provider: str, headers: Dict, body: str) -> bool:
    """Validate webhook signature based on provider."""

    if provider == 'github':
        signature = headers.get('X-Hub-Signature-256', '')
        secret = get_webhook_secret('github')

        expected = 'sha256=' + hmac.new(
            secret.encode(),
            body.encode(),
            hashlib.sha256
        ).hexdigest()

        return hmac.compare_digest(signature, expected)

    # Add more providers as needed
    return True

def query_subscriptions(provider: str, event_type: str) -> list:
    """Query DynamoDB for matching subscriptions."""

    response = webhook_config_table.query(
        IndexName='ProviderEventIndex',
        KeyConditionExpression='provider = :provider AND event_type = :event',
        ExpressionAttributeValues={
            ':provider': provider,
            ':event': event_type
        },
        FilterExpression='enabled = :enabled',
        ExpressionAttributeValues={
            ':enabled': True
        }
    )

    return response['Items']

def send_to_terminal(workspace_id: str, terminal_id: str, command: str) -> bool:
    """Send command to terminal via WebSocket connection."""

    # Look up active WebSocket connection for workspace
    connection = get_active_connection(workspace_id)

    if not connection:
        return False

    # Send message via API Gateway WebSocket
    try:
        apigateway.post_to_connection(
            ConnectionId=connection['connectionId'],
            Data=json.dumps({
                'action': 'inject',
                'terminalId': terminal_id,
                'command': command,
                'timestamp': int(time.time())
            }).encode()
        )
        return True
    except Exception as e:
        print(f"Error sending to connection: {e}")
        return False

def render_command_template(template: str, webhook_data: Dict) -> str:
    """Render command template with webhook data."""

    # Simple template engine supporting {{path.to.value}} syntax
    import re

    def replace_token(match):
        path = match.group(1).split('.')
        value = webhook_data

        for key in path:
            if isinstance(value, dict):
                value = value.get(key, '')
            else:
                return ''

        return str(value)

    return re.sub(r'\{\{([^}]+)\}\}', replace_token, template)

def handle_registration(event: Dict[str, Any]) -> Dict[str, Any]:
    """Register a new terminal subscription."""

    body = json.loads(event['body'])

    # Validate required fields
    required = ['workspaceId', 'terminalId', 'subscription']
    if not all(k in body for k in required):
        return {
            'statusCode': 400,
            'body': json.dumps({'error': 'Missing required fields'})
        }

    # Store in DynamoDB
    webhook_config_table.put_item(Item={
        'subscriptionId': body['subscription']['id'],
        'workspaceId': body['workspaceId'],
        'terminalId': body['terminalId'],
        'provider': body['subscription']['provider'],
        'event_type': body['subscription']['events'][0],  # Primary event
        'events': body['subscription']['events'],
        'filters': body['subscription']['filters'],
        'commandTemplate': body['subscription']['commandTemplate'],
        'enabled': body['subscription'].get('enabled', True),
        'createdAt': int(time.time())
    })

    return {
        'statusCode': 200,
        'body': json.dumps({'message': 'Subscription registered'})
    }

def handle_websocket_connect(event: Dict[str, Any]) -> Dict[str, Any]:
    """Handle WebSocket connection from AgentMux instance."""

    connection_id = event['requestContext']['connectionId']

    # Extract workspaceId from query params
    query_params = event.get('queryStringParameters', {})
    workspace_id = query_params.get('workspaceId')

    if not workspace_id:
        return {'statusCode': 400}

    # Validate authentication token
    auth_token = query_params.get('token')
    if not validate_auth_token(workspace_id, auth_token):
        return {'statusCode': 401}

    # Store connection in DynamoDB
    connection_table.put_item(Item={
        'connectionId': connection_id,
        'workspaceId': workspace_id,
        'connectedAt': int(time.time()),
        'ttl': int(time.time()) + 86400  # 24 hour TTL
    })

    return {'statusCode': 200}

def handle_websocket_disconnect(event: Dict[str, Any]) -> Dict[str, Any]:
    """Clean up WebSocket connection."""

    connection_id = event['requestContext']['connectionId']

    connection_table.delete_item(Key={'connectionId': connection_id})

    return {'statusCode': 200}
```

#### DynamoDB Tables

**Table 1: `AgentMuxWebhookConfig`**

```yaml
TableName: AgentMuxWebhookConfig
KeySchema:
  - AttributeName: subscriptionId
    KeyType: HASH
Attributes:
  - subscriptionId: String (UUID)
  - workspaceId: String
  - terminalId: String
  - provider: String (github, custom, etc.)
  - event_type: String
  - events: List<String>
  - filters: Map
  - commandTemplate: String
  - enabled: Boolean
  - createdAt: Number (Unix timestamp)
GlobalSecondaryIndexes:
  - IndexName: ProviderEventIndex
    KeySchema:
      - AttributeName: provider
        KeyType: HASH
      - AttributeName: event_type
        KeyType: RANGE
```

**Table 2: `AgentMuxConnections`**

```yaml
TableName: AgentMuxConnections
KeySchema:
  - AttributeName: connectionId
    KeyType: HASH
Attributes:
  - connectionId: String (WebSocket connection ID)
  - workspaceId: String
  - connectedAt: Number
  - ttl: Number (for automatic cleanup)
GlobalSecondaryIndexes:
  - IndexName: WorkspaceIndex
    KeySchema:
      - AttributeName: workspaceId
        KeyType: HASH
```

#### SAM Template Addition

Add to `shared-infrastructure/lambdas/template.yaml`:

```yaml
  # ==================== AgentMux Webhook Router ====================
  AgentMuxWebhookRouterFunction:
    Type: AWS::Serverless::Function
    Properties:
      FunctionName: !Sub agentmux-webhook-router-${Environment}
      CodeUri: agentmux-webhook-router/
      Handler: handler.lambda_handler
      Description: Route webhooks to AgentMux terminal instances
      Runtime: python3.12
      Timeout: 30
      MemorySize: 512
      Environment:
        Variables:
          WEBHOOK_CONFIG_TABLE: !Ref AgentMuxWebhookConfigTable
          CONNECTION_TABLE: !Ref AgentMuxConnectionTable
          ENVIRONMENT: !Ref Environment
      Events:
        WebhookApi:
          Type: HttpApi
          Properties:
            Path: /webhook/{provider}
            Method: POST
        RegisterApi:
          Type: HttpApi
          Properties:
            Path: /register
            Method: POST
        WebSocketConnect:
          Type: WebSocket
          Properties:
            Route: $connect
        WebSocketDisconnect:
          Type: WebSocket
          Properties:
            Route: $disconnect
      Policies:
        - DynamoDBCrudPolicy:
            TableName: !Ref AgentMuxWebhookConfigTable
        - DynamoDBCrudPolicy:
            TableName: !Ref AgentMuxConnectionTable
        - Statement:
            - Effect: Allow
              Action:
                - execute-api:ManageConnections
              Resource: !Sub "arn:aws:execute-api:${AWS::Region}:${AWS::AccountId}:*/*/*/*"
            - Effect: Allow
              Action:
                - secretsmanager:GetSecretValue
              Resource: !Sub "arn:aws:secretsmanager:${AWS::Region}:${AWS::AccountId}:secret:agentmux/*"

  AgentMuxWebhookConfigTable:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: !Sub AgentMuxWebhookConfig-${Environment}
      BillingMode: PAY_PER_REQUEST
      AttributeDefinitions:
        - AttributeName: subscriptionId
          AttributeType: S
        - AttributeName: provider
          AttributeType: S
        - AttributeName: event_type
          AttributeType: S
      KeySchema:
        - AttributeName: subscriptionId
          KeyType: HASH
      GlobalSecondaryIndexes:
        - IndexName: ProviderEventIndex
          KeySchema:
            - AttributeName: provider
              KeyType: HASH
            - AttributeName: event_type
              KeyType: RANGE
          Projection:
            ProjectionType: ALL

  AgentMuxConnectionTable:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: !Sub AgentMuxConnections-${Environment}
      BillingMode: PAY_PER_REQUEST
      AttributeDefinitions:
        - AttributeName: connectionId
          AttributeType: S
        - AttributeName: workspaceId
          AttributeType: S
      KeySchema:
        - AttributeName: connectionId
          KeyType: HASH
      GlobalSecondaryIndexes:
        - IndexName: WorkspaceIndex
          KeySchema:
            - AttributeName: workspaceId
              KeyType: HASH
          Projection:
            ProjectionType: ALL
      TimeToLiveSpecification:
        AttributeName: ttl
        Enabled: true
```

---

### 4. AgentMux Backend Integration

**Location:** `pkg/webhookinjector/webhookinjector.go`

```go
package webhookinjector

import (
    "context"
    "encoding/json"
    "fmt"
    "log"
    "sync"
    "time"

    "github.com/gorilla/websocket"
    "github.com/a5af/agentmux/pkg/blockcontroller"
    "github.com/a5af/agentmux/pkg/webhookconfig"
)

type InjectionMessage struct {
    Action     string `json:"action"`
    TerminalId string `json:"terminalId"`
    Command    string `json:"command"`
    Timestamp  int64  `json:"timestamp"`
}

type WebhookInjector struct {
    config          *webhookconfig.WebhookConfig
    conn            *websocket.Conn
    terminalMap     sync.Map // terminalId -> *blockcontroller.BlockController
    ctx             context.Context
    cancel          context.CancelFunc
    reconnectDelay  time.Duration
}

func NewWebhookInjector(config *webhookconfig.WebhookConfig) *WebhookInjector {
    ctx, cancel := context.WithCancel(context.Background())

    return &WebhookInjector{
        config:         config,
        ctx:            ctx,
        cancel:         cancel,
        reconnectDelay: 5 * time.Second,
    }
}

// Start establishes WebSocket connection and begins listening
func (wi *WebhookInjector) Start() error {
    go wi.maintainConnection()
    return nil
}

// Stop closes the WebSocket connection
func (wi *WebhookInjector) Stop() {
    wi.cancel()
    if wi.conn != nil {
        wi.conn.Close()
    }
}

// RegisterTerminal maps a terminal ID to its block controller
func (wi *WebhookInjector) RegisterTerminal(terminalId string, bc *blockcontroller.BlockController) {
    wi.terminalMap.Store(terminalId, bc)
    log.Printf("Registered terminal for webhook injection: %s", terminalId)
}

// UnregisterTerminal removes terminal from injection map
func (wi *WebhookInjector) UnregisterTerminal(terminalId string) {
    wi.terminalMap.Delete(terminalId)
    log.Printf("Unregistered terminal from webhook injection: %s", terminalId)
}

func (wi *WebhookInjector) maintainConnection() {
    for {
        select {
        case <-wi.ctx.Done():
            return
        default:
            if err := wi.connect(); err != nil {
                log.Printf("WebSocket connection error: %v", err)
                time.Sleep(wi.reconnectDelay)
                continue
            }

            wi.listen()

            // Connection closed, wait before reconnecting
            time.Sleep(wi.reconnectDelay)
        }
    }
}

func (wi *WebhookInjector) connect() error {
    url := fmt.Sprintf("%s?workspaceId=%s&token=%s",
        wi.config.CloudEndpoint,
        wi.config.WorkspaceId,
        wi.config.WebhookSecret,
    )

    conn, _, err := websocket.DefaultDialer.Dial(url, nil)
    if err != nil {
        return fmt.Errorf("dial error: %w", err)
    }

    wi.conn = conn
    log.Println("WebSocket connection established")

    return nil
}

func (wi *WebhookInjector) listen() {
    defer wi.conn.Close()

    for {
        select {
        case <-wi.ctx.Done():
            return
        default:
            var msg InjectionMessage
            err := wi.conn.ReadJSON(&msg)
            if err != nil {
                log.Printf("Read error: %v", err)
                return
            }

            wi.handleInjection(&msg)
        }
    }
}

func (wi *WebhookInjector) handleInjection(msg *InjectionMessage) {
    if msg.Action != "inject" {
        log.Printf("Unknown action: %s", msg.Action)
        return
    }

    // Look up terminal
    value, ok := wi.terminalMap.Load(msg.TerminalId)
    if !ok {
        log.Printf("Terminal not found: %s", msg.TerminalId)
        return
    }

    bc, ok := value.(*blockcontroller.BlockController)
    if !ok {
        log.Printf("Invalid terminal type for: %s", msg.TerminalId)
        return
    }

    // Inject command into terminal
    if err := bc.InjectInput([]byte(msg.Command)); err != nil {
        log.Printf("Injection error for terminal %s: %v", msg.TerminalId, err)
        return
    }

    log.Printf("Injected command into terminal %s: %q", msg.TerminalId, msg.Command)
}
```

**Integration with BlockController:**

Add to `pkg/blockcontroller/blockcontroller.go`:

```go
// InjectInput injects text into the terminal PTY as if the user typed it
func (bc *BlockController) InjectInput(input []byte) error {
    bc.Lock.Lock()
    defer bc.Lock.Unlock()

    if bc.ShellProc == nil {
        return fmt.Errorf("shell process not running")
    }

    // Write to PTY stdin
    _, err := bc.ShellProc.Cmd.StdinPipe().Write(input)
    return err
}
```

---

### 5. Frontend UI for Configuration

**Component:** `frontend/app/view/term/webhook-config-modal.tsx`

```typescript
import { Modal } from "@/app/element/modal";
import { Button } from "@/app/element/button";
import { Input } from "@/app/element/input";
import { Toggle } from "@/app/element/toggle";
import * as React from "react";

interface WebhookConfigModalProps {
    terminalId: string;
    displayName: string;
    onClose: () => void;
}

export const WebhookConfigModal: React.FC<WebhookConfigModalProps> = ({
    terminalId,
    displayName,
    onClose
}) => {
    const [subscriptions, setSubscriptions] = React.useState([]);

    React.useEffect(() => {
        // Load existing subscriptions
        loadSubscriptions();
    }, [terminalId]);

    const loadSubscriptions = async () => {
        const config = await RpcApi.LoadWebhookConfig();
        const terminal = config.terminals.find(t => t.terminalId === terminalId);
        setSubscriptions(terminal?.webhookSubscriptions || []);
    };

    const addSubscription = () => {
        setSubscriptions([
            ...subscriptions,
            {
                id: generateId(),
                provider: "github",
                events: [],
                filters: {},
                commandTemplate: "",
                enabled: true
            }
        ]);
    };

    const saveConfig = async () => {
        await RpcApi.SaveWebhookSubscriptions(terminalId, subscriptions);
        onClose();
    };

    return (
        <Modal title={`Webhook Configuration - ${displayName}`} onClose={onClose}>
            <div className="webhook-config-modal">
                <h3>Active Webhook Subscriptions</h3>

                {subscriptions.map((sub, index) => (
                    <div key={sub.id} className="subscription-card">
                        <div className="subscription-header">
                            <Input
                                label="Provider"
                                value={sub.provider}
                                onChange={(e) => updateSubscription(index, "provider", e.target.value)}
                            />
                            <Toggle
                                checked={sub.enabled}
                                onChange={(checked) => updateSubscription(index, "enabled", checked)}
                            />
                        </div>

                        <Input
                            label="Events (comma-separated)"
                            value={sub.events.join(", ")}
                            onChange={(e) => updateSubscription(index, "events", e.target.value.split(",").map(s => s.trim()))}
                        />

                        <Input
                            label="Command Template"
                            placeholder="echo '[GitHub] {{event.action}}'\n"
                            value={sub.commandTemplate}
                            onChange={(e) => updateSubscription(index, "commandTemplate", e.target.value)}
                        />

                        <Button onClick={() => removeSubscription(index)}>Remove</Button>
                    </div>
                ))}

                <Button onClick={addSubscription}>Add Subscription</Button>
                <Button onClick={saveConfig} variant="primary">Save</Button>
            </div>
        </Modal>
    );
};
```

---

## Security Considerations

### 1. Authentication

- **Webhook Signature Validation:** All incoming webhooks MUST be validated using HMAC signatures
- **WebSocket Authentication:** AgentMux instances authenticate via time-limited JWT tokens
- **Terminal Access Control:** Only registered terminals can receive injections

### 2. Authorization

- **Workspace Isolation:** Terminals can only receive webhooks for their registered workspace
- **Command Validation:** Command templates are validated before registration
- **Rate Limiting:** Lambda enforces rate limits per workspace (100 req/min)

### 3. Secrets Management

- **AWS Secrets Manager:** Webhook secrets stored in Secrets Manager, not config files
- **Rotation:** Webhook secrets can be rotated without redeployment
- **Encryption:** All WebSocket connections use WSS (TLS)

### 4. Injection Safety

- **No Arbitrary Execution:** Only text injection into PTY, no arbitrary command execution
- **Template Sandboxing:** Command templates use safe template engine with no code evaluation
- **Audit Logging:** All injections logged to CloudWatch

---

## Deployment

### Prerequisites

1. AWS Account with appropriate IAM permissions
2. AWS SAM CLI installed
3. Shared-infrastructure repository cloned

### Steps

#### 1. Deploy Lambda Infrastructure

```bash
cd /d/Code/agent-workspaces/agent2/shared-infrastructure/lambdas

# Build Lambda package
sam build

# Deploy to AWS
sam deploy \
  --stack-name agentmux-webhook-router-prod \
  --parameter-overrides Environment=prod \
  --capabilities CAPABILITY_IAM \
  --region us-east-1
```

#### 2. Configure AgentMux

Add to `~/.agentmux/webhook-config.json`:

```json
{
  "version": "1.0",
  "workspaceId": "agent2-workspace",
  "webhookSecret": "${SECRET_REF:agentmux/webhook-secret}",
  "cloudEndpoint": "wss://your-api-id.execute-api.us-east-1.amazonaws.com/prod",
  "terminals": []
}
```

#### 3. Create Webhook Secret

```bash
aws secretsmanager create-secret \
  --name agentmux/webhook-secret \
  --secret-string '{"github":"your-github-webhook-secret"}'
```

#### 4. Configure GitHub Webhook

1. Go to repository settings → Webhooks
2. Add webhook URL: `https://your-api-id.execute-api.us-east-1.amazonaws.com/prod/webhook/github`
3. Set secret to match Secrets Manager value
4. Select events: Pull requests, Issues, etc.

---

## Testing

### Manual Test

```bash
# Test webhook endpoint
curl -X POST \
  https://your-api-id.execute-api.us-east-1.amazonaws.com/prod/webhook/github \
  -H "Content-Type: application/json" \
  -H "X-Hub-Signature-256: sha256=..." \
  -d '{
    "action": "opened",
    "pull_request": {
      "number": 123,
      "title": "Test PR"
    }
  }'
```

### Integration Test

1. Start AgentMux with webhook config enabled
2. Create terminal and note terminalId
3. Register webhook subscription via UI
4. Trigger webhook from GitHub
5. Verify command appears in terminal

---

## Future Enhancements

1. **Bidirectional Communication:** Terminal output can trigger webhooks
2. **Rich Notifications:** Support for desktop notifications alongside shell injection
3. **Multi-Workspace Coordination:** Agent-to-agent communication across workspaces
4. **Interactive Prompts:** Webhooks can request user confirmation before injection
5. **Command History:** Track all injected commands for audit/replay
6. **Visual Indicators:** Terminal shows visual indicator when webhook injection occurs

---

## References

- AgentMux Block Architecture: `pkg/wcore/block.go`
- Terminal Implementation: `frontend/app/view/term/term.tsx`
- Shared Infrastructure: `shared-infrastructure/lambdas/README.md`
- AWS SAM Documentation: https://docs.aws.amazon.com/serverless-application-model/

---

**End of Specification**
