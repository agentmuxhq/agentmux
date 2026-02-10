# Native Notifications in AgentMux

AgentMux supports OS-level native notifications via `tauri-plugin-notification`, integrated with the existing in-app notification system.

## Features

- ✅ **Native OS Notifications**: System-level notifications on Windows, macOS, and Linux
- ✅ **Automatic Fallback**: Falls back to in-app notifications if native not available
- ✅ **Platform-Specific Icons**: Support for platform-specific notification icons
- ✅ **Permission Handling**: Automatic permission management
- ✅ **Common Use Cases**: Pre-built helpers for commands, tasks, errors, and updates

## Usage

### Basic Notification

```typescript
import { sendNativeNotification } from "@/util/notification";

await sendNativeNotification({
    title: "Build Complete",
    body: "Your project compiled successfully!",
});
```

### Command Completion

```typescript
import { notifyCommandComplete } from "@/util/notification";

// Notify when a long-running command finishes
await notifyCommandComplete("npm install", true, 12500); // 12.5s duration
```

Output:
```
Title: "Command Completed"
Body:  "$ npm install"
       "⏱️ 12.5s"
```

### Background Task Completion

```typescript
import { notifyTaskComplete } from "@/util/notification";

await notifyTaskComplete("Database Backup", "Backup completed: 2.4GB in 3m 42s");
```

### Error Notifications

```typescript
import { notifyError } from "@/util/notification";

await notifyError("Connection Failed", "Could not connect to backend server");
```

### Update Notifications

```typescript
import { notifyUpdate } from "@/util/notification";

await notifyUpdate("0.19.0", "A new version of AgentMux is available!");
```

## API Reference

### `sendNativeNotification(options)`

Send a native OS notification.

**Parameters:**
- `title` (string, required): The notification title
- `body` (string, optional): The notification body/message
- `icon` (string, optional): Icon path or name
- `sound` (string, optional): Notification sound
- `largeIcon` (string, optional): Large icon (Android only)
- `smallIcon` (string, optional): Small icon (Android only)

**Returns:** `Promise<void>`

### `isNativeNotificationAvailable()`

Check if native notifications are supported.

**Returns:** `Promise<boolean>` - true if Tauri notification plugin is available

### `requestNotificationPermission()`

Request notification permission (if needed by platform).

**Returns:** `Promise<"granted" | "denied" | "default">`

### Helper Functions

| Function | Purpose | Parameters |
|----------|---------|------------|
| `notifyCommandComplete` | Notify when shell command finishes | `command`, `success`, `duration?` |
| `notifyTaskComplete` | Notify when background task finishes | `taskName`, `message` |
| `notifyError` | Show error notification | `title`, `message` |
| `notifyUpdate` | Show update available notification | `version`, `message` |

## Integration Examples

### Terminal Command Notifications

Notify users when long-running commands complete in background tabs:

```typescript
// In terminal command handler
if (commandDuration > 30000) { // 30 seconds
    await notifyCommandComplete(command, exitCode === 0, commandDuration);
}
```

### Build System Integration

```typescript
// When build completes
if (buildResult.success) {
    await sendNativeNotification({
        title: "Build Successful",
        body: `Built ${buildResult.files} files in ${buildResult.time}s`,
    });
} else {
    await notifyError("Build Failed", buildResult.error);
}
```

### Update Checker Integration

```typescript
// When checking for updates
const latestVersion = await checkForUpdates();
if (latestVersion > currentVersion) {
    await notifyUpdate(latestVersion, "Click to download the latest version");
}
```

## Platform Behavior

### Windows

- Notifications appear in Action Center
- Supports custom icons and sounds
- Persists in notification history

### macOS

- Notifications appear in Notification Center
- Supports badges and actions
- Respects Do Not Disturb mode

### Linux

- Uses libnotify (notify-send)
- Supports urgency levels
- Desktop environment-specific behavior

## Best Practices

### When to Use Native Notifications

✅ **Good Use Cases:**
- Long-running command completion (when user switches tabs/windows)
- Background task completion
- Critical errors that need immediate attention
- Update notifications
- Time-sensitive alerts

❌ **Avoid Using For:**
- Routine status updates (use in-app notifications)
- High-frequency events (will spam the user)
- Non-critical informational messages
- Events when app is focused (user already sees it)

### Rate Limiting

Avoid notification spam by implementing rate limiting:

```typescript
let lastNotificationTime = 0;
const MIN_NOTIFICATION_INTERVAL = 5000; // 5 seconds

function shouldNotify(): boolean {
    const now = Date.now();
    if (now - lastNotificationTime < MIN_NOTIFICATION_INTERVAL) {
        return false;
    }
    lastNotificationTime = now;
    return true;
}

// Use in notification calls
if (shouldNotify()) {
    await sendNativeNotification({...});
}
```

### Focus Detection

Only send notifications when app is not focused:

```typescript
import { getCurrent } from "@tauri-apps/api/window";

async function notifyIfUnfocused(options: NativeNotificationOptions) {
    const currentWindow = getCurrent();
    const isFocused = await currentWindow.isFocused();

    if (!isFocused) {
        await sendNativeNotification(options);
    }
}
```

## Configuration

Notification behavior can be configured in `tauri.conf.json`:

```json
{
  "plugins": {
    "notification": {
      "identifier": "com.a5af.agentmux",
      "appName": "AgentMux"
    }
  }
}
```

## Permissions

The notification plugin requires the following capability:

```json
{
  "permissions": [
    "notification:allow-is-permission-granted",
    "notification:allow-request-permission",
    "notification:allow-notify",
    "notification:allow-show"
  ]
}
```

This is already configured in `src-tauri/capabilities/default.json`.

## Debugging

Enable notification logging:

```typescript
// In notification.ts, the module logs to console:
console.log("Native notification sent:", options.title);
console.warn("Failed to send native notification:", error);
```

Check logs in DevTools Console or `~/.waveterm-dev/waveapp.log`.

## See Also

- [Tauri Notification Plugin Docs](https://v2.tauri.app/plugin/notification/)
- [In-App Notifications](../frontend/app/store/global.ts) - AgentMux's internal notification system
- [Custom.d.ts](../frontend/types/custom.d.ts) - NotificationType definition
