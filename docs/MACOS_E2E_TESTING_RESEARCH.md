# macOS E2E Testing Tools for Tauri Frontends - Comprehensive Research Report

**Date:** February 13, 2026
**Project:** AgentMux (Tauri v2)
**Platform:** macOS (Darwin 25.2.0)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Official Tauri Testing Recommendations](#official-tauri-testing-recommendations)
3. [Tauri v2 Compatible Tools](#tauri-v2-compatible-tools)
4. [macOS-Specific Testing Solutions](#macos-specific-testing-solutions)
5. [WebDriver-Based Solutions](#webdriver-based-solutions)
6. [Alternatives to Selenium/Playwright](#alternatives-to-seleniumplaywright)
7. [Tools for User Interaction Simulation](#tools-for-user-interaction-simulation)
8. [Code Examples](#code-examples)
9. [Pros and Cons Analysis](#pros-and-cons-analysis)
10. [Recommendations](#recommendations)

---

## Executive Summary

Testing Tauri applications on macOS presents unique challenges due to the lack of native WKWebView WebDriver support. This report evaluates all available E2E testing solutions for Tauri v2 applications on macOS, including official recommendations, commercial solutions, workarounds, and alternatives.

### Key Findings

- **Critical Limitation:** Official Tauri WebDriver (tauri-driver) does NOT support macOS natively
- **Commercial Solution:** CrabNebula offers paid macOS WebDriver support via tauri-plugin-automation
- **Workarounds:** Lima VM approach allows running Linux-based testing on macOS
- **Native Alternatives:** Appium Mac2 Driver, AppleScript/JXA, and visual automation tools

---

## Official Tauri Testing Recommendations

### Testing Approaches

Tauri v2 officially supports three testing methodologies:

1. **Unit/Integration Testing** - Using @tauri-apps/api/mocks module
2. **End-to-End Testing** - Via WebDriver protocol
3. **Rust Command Testing** - Direct Rust function testing

### Official Documentation

- [Tests | Tauri](https://v2.tauri.app/develop/tests/)
- [WebDriver | Tauri](https://v2.tauri.app/develop/tests/webdriver/)
- [Mock Tauri APIs | Tauri](https://v2.tauri.app/develop/tests/mocking/)

### Platform Support Matrix

| Platform | WebDriver Support | Status |
|----------|-------------------|--------|
| Windows | msedgedriver | ✅ Supported |
| Linux | webkit2gtk-driver | ✅ Supported |
| macOS | WKWebView driver | ❌ **NOT Supported** |

### Official Testing Tools

#### 1. WebdriverIO (Recommended)

**Official Example:** [WebdriverIO | Tauri](https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/)

- Pre-configured setup with minimal configuration
- Uses local WebDriver runner
- Mocha test framework integration
- **Limitation:** Windows and Linux only

#### 2. Selenium

**Official Example:** [Selenium | Tauri](https://v2.tauri.app/develop/tests/webdriver/example/selenium/)

- Works with selenium-webdriver Node.js package
- No modification to Tauri app required
- Mocha + Chai testing stack
- **Limitation:** Windows and Linux only

### Frontend Mocking

For unit tests, Tauri provides `@tauri-apps/api/mocks`:

```typescript
import { mockWindows, clearMocks } from '@tauri-apps/api/mocks';

beforeEach(() => {
  mockWindows('main', 'settings');
});

afterEach(() => {
  clearMocks();
});
```

**Supported Test Frameworks:**
- Vitest (recommended)
- Jest
- Any JavaScript testing framework

---

## Tauri v2 Compatible Tools

### 1. CrabNebula WebDriver (Commercial)

**Provider:** CrabNebula DevTools
**Documentation:** [Integration Tests for Tauri](https://docs.crabnebula.dev/plugins/tauri-e2e-tests/)
**Package:** [@crabnebula/tauri-driver](https://www.npmjs.com/package/@crabnebula/tauri-driver)

#### Overview

CrabNebula provides the ONLY official macOS WebDriver support for Tauri applications through a custom WebDriver implementation.

#### Platform Support

| Platform | Driver | Status |
|----------|--------|--------|
| Linux | webkit2gtk-driver | ✅ Free |
| Windows | msedgedriver | ✅ Free |
| macOS | CrabNebula WebDriver for Tauri | 💰 **Requires Subscription** |

#### Setup Requirements

**1. Install tauri-plugin-automation**

```bash
cd src-tauri
cargo add tauri-plugin-automation
```

**2. Register Plugin (Debug Only)**

```rust
let mut builder = tauri::Builder::default();

#[cfg(debug_assertions)]
{
    builder = builder.plugin(tauri_plugin_automation::init());
}
```

**Critical:** Use conditional compilation to prevent plugin in production builds.

**3. Install NPM Packages**

```bash
npm install --save-dev @crabnebula/tauri-driver
npm install --save-dev @crabnebula/test-runner-backend  # For local macOS testing
npm install --save-dev @crabnebula/webdriverio-cloud-reporter  # For cloud reporting
```

#### Pricing

- macOS WebDriver requires a subscription
- Contact CrabNebula for pricing details
- Windows and Linux support is free

#### Pros

✅ Official macOS support for Tauri
✅ Seamless integration with existing WebDriver tests
✅ Works with WebdriverIO and Selenium
✅ Cross-platform (Windows, Linux, macOS)
✅ Cloud reporting capabilities

#### Cons

❌ Requires paid subscription for macOS
❌ Adds external dependency
❌ Must remember to conditionally compile plugin
❌ Pricing not publicly disclosed

---

### 2. Lima VM Approach (Free Alternative)

**Repository:** [tauri-webdriver-test](https://github.com/Kumassy/tauri-webdriver-test)
**Platform:** macOS with Lima virtualization

#### Overview

Since official WebDriver only supports Windows and Linux, this approach runs Ubuntu on macOS using Lima VM, enabling the use of webkit2gtk-driver.

#### Requirements

- XQuartz (must be installed and launched)
- Lima (Linux virtual machine runtime)
- Lima configuration file

#### Setup Process

**1. Create Lima VM**

```bash
# Start Lima VM with custom config
limactl start ./lima-config.yaml

# Access VM shell
limactl shell tauri-test
```

**2. Bootstrap Dependencies**

```bash
cd /tmp/lima/agentmux
./bootstrap.sh
```

**3. Copy Project to VM**

Due to performance issues with shared filesystems:

```bash
# Copy entire project to VM local directory
cp -r /tmp/lima/agentmux ~/agentmux-local
cd ~/agentmux-local
```

**4. Build and Test**

```bash
yarn install
yarn test
```

#### VSCode Remote Development

Use VSCode Remote-SSH extension:

```bash
# Get SSH config
limactl show-ssh tauri-test

# Add to ~/.ssh/config and connect via VSCode
```

#### Pros

✅ Free and open-source
✅ Uses official webkit2gtk-driver
✅ Full WebDriver compatibility
✅ Can test Linux-specific behavior
✅ VSCode integration available

#### Cons

❌ Complex setup process
❌ Requires virtualization overhead
❌ Slower build times (even with local copy)
❌ Not testing native macOS behavior (testing Linux)
❌ XQuartz dependency
❌ File synchronization challenges
❌ VM maintenance required

---

## macOS-Specific Testing Solutions

### 1. Appium Mac2 Driver

**Repository:** [appium-mac2-driver](https://github.com/appium/appium-mac2-driver)
**Technology:** Apple XCTest + Accessibility API

#### Overview

Next-generation Appium driver for native macOS applications, backed by Apple's XCTest framework and macOS Accessibility API.

#### System Requirements

- macOS 11 (Big Sur) or later
- Xcode 13 or later
- Xcode Helper app enabled in Accessibility settings
- `xcode-select` pointing to full Xcode (not CommandLineTools)
- May require disabling testmanagerd authentication on macOS 12+

#### Installation

```bash
# Install Appium
npm install -g appium

# Install Mac2 driver
appium driver install mac2

# Verify setup
appium driver doctor mac2
```

#### Capabilities Configuration

```javascript
const capabilities = {
  platformName: 'mac',
  automationName: 'Mac2',
  'appium:bundleId': 'com.a5af.agentmux',  // Your Tauri app bundle ID
  'appium:systemPort': 10100,
  'appium:systemHost': '127.0.0.1'
};
```

#### Custom Commands for User Interactions

Appium Mac2 provides native macOS automation commands:

**Right-Click Simulation:**

```javascript
// JavaScript/WebdriverIO
await driver.executeScript('macos: rightClick', [{
  x: 100,
  y: 200
}]);

// Or on an element
await driver.executeScript('macos: rightClick', [{
  elementId: element.elementId
}]);
```

**Context Menu Simulation:**

```javascript
// Right-click to open context menu
await driver.executeScript('macos: rightClick', [{ x: 100, y: 200 }]);

// Wait for context menu to appear
await driver.pause(500);

// Click menu item by accessibility label
const menuItem = await driver.$('~Copy');
await menuItem.click();
```

**Other Interaction Commands:**

```javascript
// Click
await driver.executeScript('macos: click', [{ x: 100, y: 200 }]);

// Double-click
await driver.executeScript('macos: doubleClick', [{ x: 100, y: 200 }]);

// Scroll
await driver.executeScript('macos: scroll', [{
  deltaX: 0,
  deltaY: 100
}]);

// Click and drag
await driver.executeScript('macos: clickAndDrag', [{
  startX: 100,
  startY: 100,
  endX: 200,
  endY: 200,
  duration: 1.0  // seconds
}]);

// With modifier keys
await driver.executeScript('macos: click', [{
  x: 100,
  y: 200,
  modifierFlags: ['command', 'shift']
}]);
```

#### Element Location Strategies

Ranked by performance:

1. **accessibilityId/id/name** (⭐⭐⭐⭐⭐) - Fastest
2. **className** - Fast element type matching
3. **predicate** - Native XCTest predicates
4. **classChain** - Flexible but optimized
5. **xpath** - Most flexible but slower

```javascript
// By accessibility ID (fastest)
const element = await driver.$('~MyButton');

// By class name
const button = await driver.$('XCUIElementTypeButton');

// By xpath (slower but flexible)
const element = await driver.$('//XCUIElementTypeButton[@label="Submit"]');
```

#### Integration with Tauri

```javascript
const { remote } = require('webdriverio');

const capabilities = {
  platformName: 'mac',
  automationName: 'Mac2',
  'appium:bundleId': 'com.a5af.agentmux'
};

async function runTest() {
  const driver = await remote({
    hostname: '127.0.0.1',
    port: 4723,
    capabilities
  });

  try {
    // Test Tauri app
    await driver.pause(2000);  // Wait for app to launch

    // Find window
    const window = await driver.$('//XCUIElementTypeWindow');

    // Right-click to open context menu
    await driver.executeScript('macos: rightClick', [{
      x: 200,
      y: 300
    }]);

    // Verify context menu appears
    const contextMenu = await driver.$('//XCUIElementTypeMenu');
    expect(await contextMenu.isDisplayed()).toBe(true);

  } finally {
    await driver.deleteSession();
  }
}
```

#### Pros

✅ Native macOS testing (tests actual macOS behavior)
✅ Uses Apple's XCTest framework
✅ Excellent context menu and right-click support
✅ Comprehensive gesture simulation
✅ Works with native macOS apps (not just webviews)
✅ WebDriver-compatible API
✅ Active development and maintenance
✅ Free and open-source

#### Cons

❌ Requires Xcode installation (large download)
❌ Accessibility permissions required
❌ Complex initial setup
❌ May need testmanagerd auth disabling
❌ Tests full macOS app, not just webview content
❌ Slower than browser-based WebDriver
❌ Element inspection can be challenging
❌ May not work with all Tauri features

---

### 2. AppleScript and JXA (JavaScript for Automation)

**Technology:** macOS native scripting
**Resources:**
- [macOS Automator MCP](https://github.com/steipete/macos-automator-mcp)
- [Using JXA with AppleScript](https://omni-automation.com/jxa-applescript.html)

#### Overview

AppleScript and JXA provide native macOS automation capabilities through the Open Scripting Architecture (OSA). Available on every Mac since 2014 without additional tools.

#### Capabilities

- UI element interaction via Accessibility API
- System Events automation
- Application scripting
- Keyboard/mouse simulation
- File system operations

#### JXA Example - Basic UI Automation

```javascript
// JavaScript for Automation
const app = Application.currentApplication();
app.includeStandardAdditions = true;

const systemEvents = Application('System Events');
const agentmux = systemEvents.processes.byName('AgentMux');

// Click button
agentmux.windows[0].buttons['Submit'].click();

// Type text
systemEvents.keystroke('hello world');

// Right-click simulation (using mouse coordinates)
const point = {x: 200, y: 300};
systemEvents.click(point, {button: 'right'});
```

#### AppleScript Example

```applescript
tell application "System Events"
    tell process "AgentMux"
        click button "Submit" of window 1

        -- Right-click at coordinates
        set {x, y} to {200, 300}
        click at {x, y} with {control down}
    end tell
end tell
```

#### Integration with Test Framework

```javascript
// Node.js integration
const { execSync } = require('child_process');

function runAppleScript(script) {
  return execSync(`osascript -e '${script}'`, { encoding: 'utf-8' });
}

// In test
describe('AgentMux Context Menu', () => {
  it('should open context menu on right-click', () => {
    runAppleScript(`
      tell application "System Events"
        tell process "AgentMux"
          click at {200, 300} with {control down}
          delay 0.5
        end tell
      end tell
    `);

    // Verify menu appeared
    const result = runAppleScript(`
      tell application "System Events"
        tell process "AgentMux"
          exists menu 1
        end tell
      end tell
    `);

    expect(result.trim()).toBe('true');
  });
});
```

#### Pros

✅ Native macOS support (built-in)
✅ No external dependencies
✅ Free and always available
✅ Direct Accessibility API access
✅ Can control any macOS app
✅ Integrates with Node.js testing frameworks
✅ Mature and stable

#### Cons

❌ Limited modern documentation
❌ Requires learning AppleScript or JXA syntax
❌ Not cross-platform
❌ Can be brittle (UI changes break tests)
❌ No WebDriver-style abstractions
❌ Difficult to debug
❌ Accessibility permissions required
❌ Less precise timing control

---

### 3. macOS Accessibility API (Direct)

**Documentation:** [Accessibility Programming Guide](https://developer.apple.com/library/archive/documentation/Accessibility/Conceptual/AccessibilityMacOSX/)

#### Overview

Direct access to macOS Accessibility API for fine-grained control over UI element interaction.

#### Key Technologies

- **AXUIElement** - Core accessibility element type
- **Accessibility Inspector** - Xcode tool for element inspection
- **Accessibility permissions** - Must be granted in System Settings

#### Swift Example

```swift
import Cocoa
import ApplicationServices

// Get running application
let app = NSWorkspace.shared.runningApplications
    .first { $0.bundleIdentifier == "com.a5af.agentmux" }

guard let app = app else { return }

// Get accessibility element
let appElement = AXUIElementCreateApplication(app.processIdentifier)

// Get window
var windowValue: AnyObject?
AXUIElementCopyAttributeValue(
    appElement,
    kAXWindowsAttribute as CFString,
    &windowValue
)

// Perform action
AXUIElementPerformAction(element, kAXPressAction as CFString)
```

#### Pros

✅ Maximum control and precision
✅ Native macOS integration
✅ No external dependencies
✅ Fast execution
✅ Can access all UI elements

#### Cons

❌ Requires Swift/Objective-C knowledge
❌ Complex API
❌ Low-level coding required
❌ Not cross-platform
❌ No test framework integration
❌ Significant development effort

---

## WebDriver-Based Solutions

### Comparison Matrix

| Solution | Platform Support | macOS Native | Cost | Maturity |
|----------|-----------------|--------------|------|----------|
| Official tauri-driver | Windows, Linux | ❌ | Free | Stable |
| CrabNebula tauri-driver | Windows, Linux, macOS | ✅ | Paid | Stable |
| Lima VM + webkit2gtk | Linux (on macOS VM) | ❌ | Free | Experimental |
| Appium Mac2 | macOS native apps | ✅ | Free | Stable |

### WebDriver Protocol Benefits

- Standardized API across tools
- Language-agnostic (JavaScript, Python, Java, etc.)
- Integration with CI/CD systems
- Parallel test execution
- Screenshot and video recording
- Network interception capabilities

### WebDriver Limitations for Tauri

1. **macOS Gap** - No official WKWebView driver from Apple
2. **WebView Focus** - Tests webview content, not native shell
3. **Platform Differences** - Behavior may vary across platforms
4. **Setup Complexity** - Requires driver installation and management

---

## Alternatives to Selenium/Playwright

### Overview

While Selenium and Playwright are excellent for browser testing, they're not ideal for native desktop applications like Tauri apps on macOS.

### Alternative Tools for 2026

#### 1. Cypress

**Website:** [Cypress.io](https://www.cypress.io/)
**Platform Support:** Windows, Linux, macOS

**Characteristics:**
- Fast execution
- Excellent debugging tools
- Developer-friendly workflow
- Time-travel debugging
- Automatic waiting

**Limitations for Tauri:**
- Designed for browser testing
- Cannot test native desktop apps
- No WebDriver protocol
- Limited to web content only

#### 2. TestCafe

**Website:** [TestCafe](https://testcafe.io/)
**Platform Support:** Windows, Linux, macOS

**Characteristics:**
- Proxy-based architecture
- No browser plugins required
- Cross-browser testing
- Easy setup

**Limitations for Tauri:**
- Browser-focused (not native apps)
- Cannot access macOS native features
- Limited desktop app support

#### 3. Puppeteer

**Website:** [Puppeteer](https://pptr.dev/)
**Platform Support:** Windows, Linux, macOS

**Characteristics:**
- Chrome DevTools Protocol
- Fast and reliable
- Excellent for headless testing
- Direct browser control

**Limitations for Tauri:**
- Chrome/Chromium only
- Tauri uses WKWebView on macOS (not Chrome)
- Cannot test native desktop behavior

#### 4. WebdriverIO

**Website:** [WebdriverIO](https://webdriver.io/)
**Platform Support:** Windows, Linux, macOS (with CrabNebula)

**Characteristics:**
- Modern WebDriver implementation
- Excellent API
- Plugin ecosystem
- Mobile and desktop support

**Best Option for Tauri:**
- Official Tauri support
- Works with CrabNebula driver
- Can test Tauri apps on all platforms
- Active community

#### 5. Katalon

**Website:** [Katalon](https://katalon.com/)
**Type:** Commercial with free tier

**Characteristics:**
- Multi-platform testing
- Web, mobile, API, desktop support
- Record and playback
- CI/CD integration

**Limitations:**
- Commercial product
- Heavyweight solution
- May not support Tauri natively

---

## Tools for User Interaction Simulation

### Right-Click and Context Menu Testing

| Tool | Right-Click Support | Context Menu Detection | macOS Support | Code Example |
|------|-------------------|----------------------|---------------|--------------|
| Appium Mac2 | ✅ Excellent | ✅ Full | ✅ Native | `macos: rightClick` |
| AppleScript/JXA | ✅ Good | ⚠️ Limited | ✅ Native | `click with {control down}` |
| CrabNebula WebDriver | ⚠️ Via JS events | ⚠️ Via JS events | ✅ WebView | WebDriver actions |
| Visual Automation | ✅ Coordinate-based | ❌ None | ✅ Works | Image matching |

### Detailed Tool Analysis

#### 1. Appium Mac2 Driver (Best for Native Interactions)

**Right-Click:**

```javascript
// Coordinate-based
await driver.executeScript('macos: rightClick', [{ x: 200, y: 300 }]);

// Element-based
const element = await driver.$('~MyElement');
await driver.executeScript('macos: rightClick', [{
  elementId: element.elementId
}]);
```

**Context Menu Interaction:**

```javascript
// Open context menu
await driver.executeScript('macos: rightClick', [{ x: 200, y: 300 }]);

// Wait for menu
await driver.pause(500);

// Select menu item by accessibility label
const menuItem = await driver.$('~Copy');
await menuItem.click();

// Or by xpath
const menuItem = await driver.$('//XCUIElementTypeMenuItem[@label="Paste"]');
await menuItem.click();
```

**Drag and Drop:**

```javascript
await driver.executeScript('macos: clickAndDrag', [{
  startX: 100,
  startY: 100,
  endX: 300,
  endY: 300,
  duration: 1.0
}]);
```

#### 2. WebdriverIO with Browser Context

When testing webview content (not native UI):

```javascript
// Using WebDriver Actions API
const actions = [
  {
    type: 'pointer',
    id: 'mouse',
    parameters: { pointerType: 'mouse' },
    actions: [
      { type: 'pointerMove', duration: 0, x: 200, y: 300 },
      { type: 'pointerDown', button: 2 },  // Right button
      { type: 'pointerUp', button: 2 }
    ]
  }
];

await driver.performActions(actions);

// Or using JavaScript injection
await driver.execute((x, y) => {
  const element = document.elementFromPoint(x, y);
  const event = new MouseEvent('contextmenu', {
    bubbles: true,
    cancelable: true,
    view: window,
    button: 2
  });
  element.dispatchEvent(event);
}, 200, 300);
```

#### 3. Visual Automation Tools

**Sikuli / SikuliX**

```python
# Image-based automation
click("right_click_target.png")
rightClick("element.png")

# Wait for context menu
wait("context_menu.png", 5)

# Click menu item
click("copy_menu_item.png")
```

**PyAutoGUI**

```python
import pyautogui

# Right-click at coordinates
pyautogui.rightClick(200, 300)

# Wait for context menu
pyautogui.sleep(0.5)

# Click menu item (relative to context menu)
pyautogui.click(220, 320)
```

**Pros of Visual Automation:**
- Works with any application
- No API dependencies
- Cross-platform
- Simple to understand

**Cons of Visual Automation:**
- Brittle (breaks with UI changes)
- Resolution-dependent
- Slow execution
- Difficult to maintain
- No element inspection

---

## Code Examples

### Complete Testing Setup Examples

#### Example 1: WebdriverIO + CrabNebula (macOS)

**Directory Structure:**

```
agentmux/
├── src-tauri/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── e2e-tests/
│   ├── package.json
│   ├── wdio.conf.js
│   └── specs/
│       └── app.spec.js
```

**package.json:**

```json
{
  "name": "agentmux-e2e-tests",
  "version": "1.0.0",
  "scripts": {
    "test": "wdio run wdio.conf.js"
  },
  "devDependencies": {
    "@wdio/cli": "^8.0.0",
    "@wdio/local-runner": "^8.0.0",
    "@wdio/mocha-framework": "^8.0.0",
    "@wdio/spec-reporter": "^8.0.0",
    "@crabnebula/tauri-driver": "^0.4.0",
    "@crabnebula/test-runner-backend": "^0.1.0"
  }
}
```

**src-tauri/Cargo.toml:**

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "devtools"] }
tauri-plugin-automation = { version = "0.1", optional = true }

[features]
default = []
automation = ["dep:tauri-plugin-automation"]
```

**src-tauri/src/main.rs:**

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Only include automation plugin in debug builds
    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_automation::init());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**wdio.conf.js:**

```javascript
const os = require('os');
const path = require('path');
const { spawn } = require('child_process');

const application = os.platform() === 'darwin'
  ? './src-tauri/target/debug/bundle/macos/AgentMux.app/Contents/MacOS/AgentMux'
  : './src-tauri/target/debug/agentmux';

let tauriDriver;

exports.config = {
  specs: ['./specs/**/*.js'],
  maxInstances: 1,
  capabilities: [{
    'tauri:options': {
      application
    }
  }],
  runner: 'local',
  port: 4444,
  host: '127.0.0.1',
  logLevel: 'info',
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    timeout: 60000
  },

  // Lifecycle hooks
  onPrepare: () => {
    // Build Tauri app
    const { execSync } = require('child_process');
    execSync('cd src-tauri && cargo build', { stdio: 'inherit' });
  },

  beforeSession: () => {
    // Start tauri-driver
    tauriDriver = spawn('tauri-driver', [], {
      stdio: [null, process.stdout, process.stderr]
    });
  },

  afterSession: () => {
    // Stop tauri-driver
    if (tauriDriver) {
      tauriDriver.kill();
    }
  }
};
```

**specs/app.spec.js:**

```javascript
describe('AgentMux Application', () => {
  it('should launch and show main window', async () => {
    // Wait for app to initialize
    await browser.pause(2000);

    // Find window title
    const title = await browser.getTitle();
    expect(title).toBe('AgentMux');
  });

  it('should interact with terminal', async () => {
    // Find terminal element
    const terminal = await browser.$('.xterm-viewport');
    await expect(terminal).toBeDisplayed();

    // Click terminal
    await terminal.click();

    // Type command (via JavaScript injection for webview content)
    await browser.execute(() => {
      const event = new KeyboardEvent('keydown', { key: 'l', ctrlKey: true });
      document.querySelector('.xterm-viewport').dispatchEvent(event);
    });
  });

  it('should open and interact with context menu', async () => {
    const terminal = await browser.$('.xterm-viewport');

    // Right-click on terminal (WebDriver actions)
    await browser.performActions([{
      type: 'pointer',
      id: 'mouse',
      parameters: { pointerType: 'mouse' },
      actions: [
        { type: 'pointerMove', duration: 0, origin: terminal },
        { type: 'pointerDown', button: 2 },
        { type: 'pointerUp', button: 2 }
      ]
    }]);

    // Wait for context menu
    await browser.pause(500);

    // Find context menu (depends on your implementation)
    const contextMenu = await browser.$('.context-menu');
    await expect(contextMenu).toBeDisplayed();
  });
});
```

---

#### Example 2: Appium Mac2 Driver (Native macOS Testing)

**package.json:**

```json
{
  "name": "agentmux-appium-tests",
  "version": "1.0.0",
  "scripts": {
    "test": "mocha test/**/*.spec.js"
  },
  "devDependencies": {
    "appium": "^2.0.0",
    "appium-mac2-driver": "^2.0.0",
    "webdriverio": "^8.0.0",
    "mocha": "^10.0.0",
    "chai": "^4.0.0"
  }
}
```

**test/agentmux.spec.js:**

```javascript
const { remote } = require('webdriverio');
const { expect } = require('chai');

describe('AgentMux Native macOS Tests', () => {
  let driver;

  before(async () => {
    // Ensure app is built
    const { execSync } = require('child_process');
    execSync('cd src-tauri && cargo build', { stdio: 'inherit' });

    // Launch app first (outside Appium)
    execSync('open src-tauri/target/debug/bundle/macos/AgentMux.app');

    // Wait for app to launch
    await new Promise(resolve => setTimeout(resolve, 3000));

    // Connect to running app
    driver = await remote({
      hostname: '127.0.0.1',
      port: 4723,
      capabilities: {
        platformName: 'mac',
        automationName: 'Mac2',
        'appium:bundleId': 'com.a5af.agentmux'
      }
    });
  });

  after(async () => {
    if (driver) {
      await driver.deleteSession();
    }
  });

  it('should find application window', async () => {
    const window = await driver.$('//XCUIElementTypeWindow[1]');
    expect(await window.isDisplayed()).to.be.true;
  });

  it('should right-click and open context menu', async () => {
    // Get window bounds first
    const window = await driver.$('//XCUIElementTypeWindow[1]');
    const bounds = await window.getAttribute('frame');

    // Calculate center point
    const x = bounds.x + bounds.width / 2;
    const y = bounds.y + bounds.height / 2;

    // Right-click at center
    await driver.executeScript('macos: rightClick', [{ x, y }]);

    // Wait for context menu
    await driver.pause(500);

    // Verify context menu appeared
    const menu = await driver.$('//XCUIElementTypeMenu');
    expect(await menu.isExisting()).to.be.true;
  });

  it('should interact with menu items', async () => {
    // Right-click to open menu
    await driver.executeScript('macos: rightClick', [{ x: 400, y: 300 }]);
    await driver.pause(500);

    // Find specific menu item
    const copyItem = await driver.$('//XCUIElementTypeMenuItem[@label="Copy"]');

    if (await copyItem.isExisting()) {
      await copyItem.click();
    }
  });

  it('should perform drag and drop', async () => {
    // Drag from one point to another
    await driver.executeScript('macos: clickAndDrag', [{
      startX: 200,
      startY: 200,
      endX: 400,
      endY: 400,
      duration: 1.0
    }]);

    // Verify drag result (depends on your app)
    await driver.pause(1000);
  });

  it('should use keyboard shortcuts', async () => {
    // Focus window first
    const window = await driver.$('//XCUIElementTypeWindow[1]');
    await window.click();

    // Send Cmd+N (new window/tab)
    await driver.executeScript('macos: keys', [{
      keys: ['command', 'n']
    }]);

    await driver.pause(1000);
  });
});
```

**Starting Appium Server:**

```bash
# Terminal 1: Start Appium
appium server

# Terminal 2: Run tests
npm test
```

---

#### Example 3: AppleScript Integration Tests

**test/applescript-tests.js:**

```javascript
const { execSync } = require('child_process');
const { expect } = require('chai');

function runAppleScript(script) {
  try {
    return execSync(`osascript -e '${script.replace(/'/g, "'\"'\"'")}'`, {
      encoding: 'utf-8'
    });
  } catch (error) {
    throw new Error(`AppleScript failed: ${error.message}`);
  }
}

function runJXA(jsCode) {
  try {
    const escaped = jsCode.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
    return execSync(`osascript -l JavaScript -e "${escaped}"`, {
      encoding: 'utf-8'
    });
  } catch (error) {
    throw new Error(`JXA failed: ${error.message}`);
  }
}

describe('AgentMux AppleScript Tests', () => {
  before(() => {
    // Launch app
    runAppleScript('tell application "AgentMux" to activate');
    execSync('sleep 2');  // Wait for launch
  });

  after(() => {
    // Quit app
    runAppleScript('tell application "AgentMux" to quit');
  });

  it('should verify app is running', () => {
    const result = runAppleScript(`
      tell application "System Events"
        exists process "AgentMux"
      end tell
    `);
    expect(result.trim()).to.equal('true');
  });

  it('should interact with main window', () => {
    const windowCount = runAppleScript(`
      tell application "System Events"
        tell process "AgentMux"
          count windows
        end tell
      end tell
    `);
    expect(parseInt(windowCount)).to.be.greaterThan(0);
  });

  it('should simulate right-click with JXA', () => {
    const jxaCode = `
      const systemEvents = Application('System Events');
      const agentmux = systemEvents.processes.byName('AgentMux');
      const window = agentmux.windows[0];

      // Get window position
      const pos = window.position();
      const size = window.size();

      // Click center with control key (right-click)
      const x = pos[0] + size[0] / 2;
      const y = pos[1] + size[1] / 2;

      systemEvents.click(systemEvents.at(x, y), {
        using: 'control down'
      });

      'success';
    `;

    const result = runJXA(jxaCode);
    expect(result.trim()).to.include('success');
  });

  it('should type text into focused element', () => {
    runAppleScript(`
      tell application "System Events"
        tell process "AgentMux"
          keystroke "ls -la"
          key code 36  -- Return key
        end tell
      end tell
    `);

    execSync('sleep 1');
    // Verify command was executed (depends on your app)
  });
});
```

---

#### Example 4: Hybrid Approach (WebDriver + AppleScript)

Combine WebDriver for webview content with AppleScript for native interactions:

```javascript
const { remote } = require('webdriverio');
const { execSync } = require('child_process');

describe('Hybrid Testing Approach', () => {
  let driver;

  before(async () => {
    // Start tauri-driver (CrabNebula or Lima VM)
    // ... driver setup ...

    driver = await remote({
      hostname: '127.0.0.1',
      port: 4444,
      capabilities: {
        'tauri:options': {
          application: './src-tauri/target/debug/AgentMux.app'
        }
      }
    });
  });

  it('should test webview content with WebDriver', async () => {
    // Test web content
    const terminal = await driver.$('.xterm-viewport');
    await expect(terminal).toBeDisplayed();
  });

  it('should test native features with AppleScript', () => {
    // Test native macOS features
    const result = execSync(`osascript -e '
      tell application "System Events"
        tell process "AgentMux"
          exists menu bar 1
        end tell
      end tell
    '`, { encoding: 'utf-8' });

    expect(result.trim()).to.equal('true');
  });
});
```

---

## Pros and Cons Analysis

### Comprehensive Tool Comparison

#### CrabNebula WebDriver + tauri-plugin-automation

**Pros:**
- ✅ Official macOS support for Tauri
- ✅ Seamless cross-platform testing (Windows, Linux, macOS)
- ✅ WebDriver-compatible (standard API)
- ✅ Works with existing WebdriverIO/Selenium tests
- ✅ Professional support available
- ✅ Cloud reporting and CI/CD integration
- ✅ Tests webview content accurately

**Cons:**
- ❌ Requires paid subscription for macOS
- ❌ Pricing not publicly disclosed
- ❌ External dependency (vendor lock-in risk)
- ❌ Must conditionally compile plugin
- ❌ Plugin must not be in production builds
- ❌ Only tests webview, not native shell
- ❌ Additional complexity in build process

**Best For:** Teams with budget, need for official support, cross-platform testing requirements

**Cost:** Unknown subscription fee (contact CrabNebula)

---

#### Lima VM + webkit2gtk-driver

**Pros:**
- ✅ Completely free and open-source
- ✅ Uses official Tauri WebDriver
- ✅ Full WebDriver compatibility
- ✅ Can test Linux-specific behavior
- ✅ VSCode Remote-SSH integration
- ✅ No vendor dependency
- ✅ No code changes to app required

**Cons:**
- ❌ Complex setup process
- ❌ Requires XQuartz
- ❌ VM overhead (performance impact)
- ❌ Not testing macOS-specific behavior
- ❌ File synchronization challenges
- ❌ Slower build times
- ❌ VM maintenance required
- ❌ Development workflow friction
- ❌ Tests Linux WebView (not WKWebView)

**Best For:** Open-source projects, teams without budget, Linux testing needs

**Cost:** Free

---

#### Appium Mac2 Driver

**Pros:**
- ✅ Native macOS testing (real WKWebView)
- ✅ Apple XCTest framework backing
- ✅ Excellent UI interaction support
- ✅ Right-click and gesture simulation
- ✅ Context menu testing capabilities
- ✅ Free and open-source
- ✅ Active development
- ✅ WebDriver-compatible API
- ✅ Tests full application (native + webview)
- ✅ Professional documentation

**Cons:**
- ❌ Requires Xcode (8+ GB download)
- ❌ Complex initial setup
- ❌ Accessibility permissions required
- ❌ May need testmanagerd auth disabling
- ❌ Slower than browser WebDriver
- ❌ Element inspection can be challenging
- ❌ Tests at application level (not just webview)
- ❌ May have Tauri-specific compatibility issues
- ❌ Learning curve for native app testing

**Best For:** Native macOS behavior testing, comprehensive E2E tests, teams comfortable with Xcode

**Cost:** Free

---

#### AppleScript / JXA

**Pros:**
- ✅ Built into macOS (no installation)
- ✅ Completely free
- ✅ Native macOS integration
- ✅ Direct Accessibility API access
- ✅ Can control any macOS app
- ✅ Integrates with Node.js
- ✅ No external dependencies
- ✅ Mature and stable

**Cons:**
- ❌ Limited modern documentation
- ❌ Requires learning new syntax
- ❌ Not cross-platform
- ❌ Can be brittle
- ❌ No WebDriver abstractions
- ❌ Difficult to debug
- ❌ Requires Accessibility permissions
- ❌ Less precise timing
- ❌ Difficult to inspect elements

**Best For:** Quick automation scripts, macOS-specific features, no-budget projects

**Cost:** Free

---

#### Visual Automation (Sikuli, PyAutoGUI)

**Pros:**
- ✅ Works with any application
- ✅ No API dependencies
- ✅ Cross-platform
- ✅ Simple to understand
- ✅ Quick to prototype
- ✅ Free and open-source

**Cons:**
- ❌ Extremely brittle
- ❌ Resolution-dependent
- ❌ Slow execution
- ❌ Difficult to maintain
- ❌ No element inspection
- ❌ Poor error messages
- ❌ Breaks with UI changes
- ❌ Not suitable for CI/CD

**Best For:** Proof of concepts, one-off automation, desktop apps with no API access

**Cost:** Free

---

### Decision Matrix

| Requirement | CrabNebula | Lima VM | Appium Mac2 | AppleScript | Visual |
|-------------|------------|---------|-------------|-------------|--------|
| macOS Native Testing | ⚠️ WebView | ❌ Linux | ✅ Yes | ✅ Yes | ✅ Yes |
| Free/Open Source | ❌ Paid | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| WebDriver API | ✅ Yes | ✅ Yes | ✅ Yes | ❌ No | ❌ No |
| Easy Setup | ✅ Easy | ❌ Complex | ⚠️ Moderate | ✅ Easy | ✅ Easy |
| Cross-Platform | ✅ Yes | ⚠️ Linux only | ❌ macOS only | ❌ macOS only | ⚠️ Limited |
| CI/CD Ready | ✅ Yes | ⚠️ Complex | ✅ Yes | ⚠️ Limited | ❌ No |
| Right-Click Support | ⚠️ JS Events | ⚠️ JS Events | ✅ Native | ✅ Native | ✅ Coordinate |
| Context Menu Testing | ⚠️ Limited | ⚠️ Limited | ✅ Excellent | ✅ Good | ❌ Poor |
| Maintenance Burden | ✅ Low | ❌ High | ⚠️ Moderate | ⚠️ Moderate | ❌ Very High |
| Learning Curve | ✅ Low | ⚠️ Moderate | ⚠️ Moderate | ⚠️ Moderate | ✅ Low |
| Production Ready | ✅ Yes | ❌ No | ✅ Yes | ⚠️ Limited | ❌ No |

---

## Recommendations

### For AgentMux Project Specifically

Given AgentMux's current state and requirements:

#### Primary Recommendation: Appium Mac2 Driver

**Rationale:**
1. AgentMux is currently macOS-only (no immediate cross-platform needs)
2. Native macOS testing is critical for terminal application
3. Free and open-source aligns with project goals
4. Excellent support for context menus and right-clicks
5. Can test native window management features
6. WebDriver-compatible API for future expansion

**Implementation Plan:**

1. **Phase 1: Setup (Week 1)**
   - Install Xcode and Appium
   - Configure Mac2 driver
   - Set up test directory structure
   - Create basic test suite

2. **Phase 2: Core Tests (Week 2-3)**
   - Window management tests
   - Terminal interaction tests
   - Context menu tests
   - Keyboard shortcut tests

3. **Phase 3: Integration (Week 4)**
   - CI/CD integration
   - Test reporting
   - Coverage measurement
   - Documentation

#### Secondary Recommendation: AppleScript/JXA for Quick Tests

**Use Cases:**
- Smoke tests before releases
- Quick manual test automation
- Native macOS feature verification
- Development workflow automation

**Integration:**
```javascript
// In npm scripts
"scripts": {
  "test:e2e": "mocha test/appium/**/*.spec.js",
  "test:smoke": "node test/applescript/smoke-tests.js",
  "test:all": "npm run test && npm run test:e2e && npm run test:smoke"
}
```

#### Future Consideration: CrabNebula (When Cross-Platform)

If/when AgentMux expands to Windows/Linux:
- Evaluate CrabNebula subscription cost
- Migrate Appium tests to WebdriverIO
- Add tauri-plugin-automation
- Maintain Appium tests for macOS-specific features

### General Recommendations by Use Case

#### Scenario 1: Cross-Platform Tauri App with Budget

**Recommended:** CrabNebula WebDriver + WebdriverIO

**Setup:**
```bash
npm install --save-dev @crabnebula/tauri-driver
npm install --save-dev @wdio/cli @wdio/local-runner
cargo add tauri-plugin-automation
```

**Benefits:**
- Official support
- Cross-platform consistency
- Professional tooling
- CI/CD ready

---

#### Scenario 2: macOS-Only App, No Budget

**Recommended:** Appium Mac2 Driver

**Setup:**
```bash
npm install -g appium
appium driver install mac2
npm install --save-dev webdriverio mocha chai
```

**Benefits:**
- Free and open-source
- Native macOS testing
- Excellent interaction support
- Production-ready

---

#### Scenario 3: Quick Prototyping / Proof of Concept

**Recommended:** AppleScript/JXA

**Setup:**
```bash
# No installation required!
osascript -e 'tell application "System Events" to ...'
```

**Benefits:**
- Zero setup time
- Built into macOS
- Quick results
- No dependencies

---

#### Scenario 4: Need Linux Testing on macOS

**Recommended:** Lima VM + webkit2gtk-driver

**Setup:**
```bash
brew install lima
limactl start ./lima-config.yaml
# Follow tauri-webdriver-test setup
```

**Benefits:**
- Free solution
- Official Tauri driver
- Test Linux-specific behavior

---

### Testing Strategy Recommendation

**Layered Approach:**

```
┌─────────────────────────────────────┐
│   Visual Smoke Tests                │ ← Quick manual verification
│   (AppleScript/JXA)                 │
├─────────────────────────────────────┤
│   E2E Tests (Appium Mac2)           │ ← Primary automated tests
│   - Window management                │
│   - User interactions                │
│   - Context menus                    │
├─────────────────────────────────────┤
│   Integration Tests (Vitest)        │ ← Current tests
│   - Layout logic                     │
│   - State management                 │
├─────────────────────────────────────┤
│   Unit Tests (Vitest)                │ ← Current tests
│   - Component logic                  │
│   - Utilities                        │
└─────────────────────────────────────┘
```

**Test Coverage Goals:**

1. **Unit Tests (Current: 39 tests)** → Target: 100+ tests
   - Cover critical utilities
   - Test pure functions
   - Fast execution (<5s)

2. **Integration Tests** → Target: 20+ tests
   - Layout model interactions
   - Block management
   - Connection handling

3. **E2E Tests (New)** → Target: 15-20 tests
   - App launch and shutdown
   - Window operations
   - Terminal interactions
   - Context menus
   - Keyboard shortcuts
   - File operations

4. **Smoke Tests (New)** → Target: 5-10 tests
   - App launches successfully
   - Core features work
   - No critical errors
   - Version display correct

---

### Implementation Priority

**Phase 1: Foundation (Current Sprint)**
- [ ] Set up Appium Mac2 driver
- [ ] Create test directory structure
- [ ] Write 3-5 basic E2E tests
- [ ] Document testing process

**Phase 2: Core Coverage (Next Sprint)**
- [ ] Add window management tests
- [ ] Add context menu tests
- [ ] Add keyboard shortcut tests
- [ ] Achieve 80% critical path coverage

**Phase 3: CI/CD Integration (Future)**
- [ ] Automate test execution
- [ ] Set up test reporting
- [ ] Add coverage tracking
- [ ] Create pre-release smoke tests

**Phase 4: Advanced (Future)**
- [ ] Performance testing
- [ ] Screenshot comparison
- [ ] Video recording
- [ ] Parallel test execution

---

## Additional Resources

### Official Documentation

- [Tauri v2 Testing Guide](https://v2.tauri.app/develop/tests/)
- [Tauri WebDriver Documentation](https://v2.tauri.app/develop/tests/webdriver/)
- [CrabNebula E2E Testing](https://docs.crabnebula.dev/plugins/tauri-e2e-tests/)
- [Appium Mac2 Driver Documentation](https://github.com/appium/appium-mac2-driver)
- [WebdriverIO Documentation](https://webdriver.io/)

### Example Repositories

- [tauri-webdriver-example](https://github.com/tauri-apps/webdriver-example) - Official Tauri example
- [tauri-webdriver-test](https://github.com/Kumassy/tauri-webdriver-test) - Lima VM approach
- [tauri-e2e](https://github.com/bukowa/tauri-e2e) - Selenium example
- [tauri-template](https://github.com/dannysmith/tauri-template) - Production template

### Tools and Libraries

- [WebdriverIO](https://webdriver.io/) - Modern WebDriver client
- [Appium](https://appium.io/) - Cross-platform automation
- [Selenium](https://www.selenium.dev/) - Browser automation
- [Mocha](https://mochajs.org/) - Test framework
- [Chai](https://www.chaijs.com/) - Assertion library
- [Vitest](https://vitest.dev/) - Unit test framework

### Learning Resources

- [WebDriver W3C Specification](https://w3c.github.io/webdriver/)
- [macOS Accessibility Programming Guide](https://developer.apple.com/library/archive/documentation/Accessibility/Conceptual/AccessibilityMacOSX/)
- [AppleScript Language Guide](https://developer.apple.com/library/archive/documentation/AppleScript/Conceptual/AppleScriptLangGuide/)
- [JavaScript for Automation Guide](https://omni-automation.com/jxa-applescript.html)

---

## Conclusion

Testing Tauri applications on macOS requires careful consideration of trade-offs between cost, complexity, and testing scope. While the official Tauri WebDriver doesn't support macOS, several viable alternatives exist:

1. **CrabNebula** - Best for teams with budget and cross-platform needs
2. **Appium Mac2** - Best for macOS-focused apps requiring native testing
3. **Lima VM** - Best for open-source projects needing free solution
4. **AppleScript/JXA** - Best for quick automation and smoke tests

For AgentMux specifically, **Appium Mac2 Driver** is recommended as the primary E2E testing solution, supplemented with AppleScript for quick smoke tests. This provides comprehensive native macOS testing capabilities while maintaining zero licensing costs.

### Next Steps for AgentMux

1. Install and configure Appium Mac2 Driver
2. Create initial test suite covering:
   - Window management
   - Terminal interactions
   - Context menus
   - Keyboard shortcuts
3. Integrate with existing Vitest unit tests
4. Document testing procedures
5. Set up CI/CD integration (future phase)

---

**Document Version:** 1.0
**Last Updated:** February 13, 2026
**Maintained By:** AgentMux Development Team

## Sources

- [Tests | Tauri](https://v2.tauri.app/develop/tests/)
- [WebDriver | Tauri](https://v2.tauri.app/develop/tests/webdriver/)
- [WebdriverIO | Tauri](https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/)
- [Selenium | Tauri](https://v2.tauri.app/develop/tests/webdriver/example/selenium/)
- [Integration Tests for Tauri | Docs](https://docs.crabnebula.dev/plugins/tauri-e2e-tests/)
- [@crabnebula/tauri-driver - npm](https://www.npmjs.com/package/@crabnebula/tauri-driver)
- [GitHub - Kumassy/tauri-webdriver-test](https://github.com/Kumassy/tauri-webdriver-test)
- [GitHub - appium/appium-mac2-driver](https://github.com/appium/appium-mac2-driver)
- [8 Top Playwright Alternatives in 2026](https://testgrid.io/blog/playwright-alternatives/)
- [Playwright vs Selenium : Which to choose in 2026](https://www.browserstack.com/guide/playwright-vs-selenium)
- [macOS Automator MCP](https://github.com/steipete/macos-automator-mcp)
- [Using Omni Automation with JXA and AppleScript](https://omni-automation.com/jxa-applescript.html)
- [Parsing macOS application UI: Techniques and tools](https://research.macpaw.com/publications/how-to-parse-macos-app-ui)
- [Performing accessibility testing for your app | Apple Developer](https://developer.apple.com/documentation/accessibility/performing-accessibility-testing-for-your-app)
- [Top 12 Alternatives to PyAutoGUI](https://testdriver.ai/articles/top-12-alternatives-to-pyautogui-for-windows-macos-linux-testing)
- [Top 15 Alternatives to SikuliX](https://testdriver.ai/articles/top-15-alternatives-to-sikulix-for-linux-windows-macos-testing)
