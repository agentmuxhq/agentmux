/**
 * Helper functions for Tauri testing
 */

/**
 * Invoke a Tauri command and return the result
 */
export async function invokeTauriCommand(commandName, args = {}) {
  return await browser.executeAsync(async (cmd, cmdArgs, done) => {
    try {
      const { invoke } = window.__TAURI_INTERNALS__
      const result = await invoke(cmd, cmdArgs)
      done({ success: true, result })
    } catch (error) {
      done({ success: false, error: error.message })
    }
  }, commandName, args)
}

/**
 * Get current zoom factor from Tauri
 */
export async function getZoomFactor() {
  const response = await invokeTauriCommand('get_zoom_factor')
  if (response.success) {
    return response.result
  }
  throw new Error(`Failed to get zoom factor: ${response.error}`)
}

/**
 * Set zoom factor via Tauri
 */
export async function setZoomFactor(factor) {
  const response = await invokeTauriCommand('set_zoom_factor', { factor })
  if (!response.success) {
    throw new Error(`Failed to set zoom factor: ${response.error}`)
  }
}

/**
 * Wait for an element to be ready
 */
export async function waitForElement(selector, timeout = 10000) {
  await browser.waitUntil(
    async () => {
      const element = await $(selector)
      return await element.isDisplayed()
    },
    {
      timeout,
      timeoutMsg: `Element ${selector} did not become ready`
    }
  )
}

/**
 * Capture screenshot with timestamp
 */
export async function captureScreenshot(name) {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-')
  const filename = `${name}-${timestamp}.png`
  await browser.saveScreenshot(`./test/screenshots/${filename}`)
  console.log(`Screenshot saved: ${filename}`)
  return filename
}

/**
 * Get Tauri version
 */
export async function getTauriVersion() {
  const response = await invokeTauriCommand('get_about_modal_details')
  if (response.success) {
    return response.result.version
  }
  throw new Error('Failed to get version')
}

/**
 * Get current platform
 */
export async function getPlatform() {
  return await browser.execute(() => {
    return navigator.platform
  })
}

/**
 * Wait for application to be fully loaded
 */
export async function waitForAppReady(timeout = 30000) {
  await browser.waitUntil(
    async () => {
      const isReady = await browser.execute(() => {
        return window.__TAURI_INTERNALS__ !== undefined &&
               document.readyState === 'complete'
      })
      return isReady
    },
    {
      timeout,
      timeoutMsg: 'Application did not become ready'
    }
  )
  console.log('Application is ready')
}
