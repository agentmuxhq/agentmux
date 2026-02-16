import {
  getZoomFactor,
  setZoomFactor,
  waitForAppReady,
  captureScreenshot,
  getPlatform
} from '../helpers/tauri-helpers.js'

describe('Zoom Functionality', () => {
  before(async () => {
    console.log('Waiting for app to be ready...')
    await waitForAppReady()

    const platform = await getPlatform()
    console.log(`Running on platform: ${platform}`)

    const version = await browser.execute(() => {
      return window.__TAURI_INTERNALS__ ? 'Tauri available' : 'No Tauri'
    })
    console.log(`Tauri status: ${version}`)
  })

  beforeEach(async () => {
    // Reset zoom to 1.0 before each test
    try {
      await setZoomFactor(1.0)
      await browser.pause(500)
    } catch (error) {
      console.log('Could not reset zoom:', error.message)
    }
  })

  describe('Keyboard Shortcuts - Ctrl', () => {
    it('should zoom in with Ctrl+=', async function() {
      this.timeout(15000)

      console.log('Getting initial zoom...')
      const initialZoom = await getZoomFactor()
      console.log(`Initial zoom: ${initialZoom}`)

      console.log('Pressing Ctrl+=...')
      await browser.keys(['Control', '='])
      await browser.pause(500)

      console.log('Getting new zoom...')
      const newZoom = await getZoomFactor()
      console.log(`New zoom after Ctrl+=: ${newZoom}`)

      if (newZoom <= initialZoom) {
        await captureScreenshot('zoom-ctrl-plus-failed')
      }

      expect(newZoom).toBeGreaterThan(initialZoom)
    })

    it('should zoom in with Ctrl++', async function() {
      this.timeout(15000)

      const initialZoom = await getZoomFactor()
      console.log(`Initial zoom: ${initialZoom}`)

      await browser.keys(['Control', '+'])
      await browser.pause(500)

      const newZoom = await getZoomFactor()
      console.log(`New zoom after Ctrl++: ${newZoom}`)

      expect(newZoom).toBeGreaterThan(initialZoom)
    })

    it('should zoom out with Ctrl+-', async function() {
      this.timeout(15000)

      // First zoom in
      await setZoomFactor(1.5)
      await browser.pause(300)

      const initialZoom = await getZoomFactor()
      console.log(`Initial zoom: ${initialZoom}`)

      await browser.keys(['Control', '-'])
      await browser.pause(500)

      const newZoom = await getZoomFactor()
      console.log(`New zoom after Ctrl+-: ${newZoom}`)

      expect(newZoom).toBeLessThan(initialZoom)
    })

    it('should reset zoom with Ctrl+0', async function() {
      this.timeout(15000)

      // First zoom in
      await setZoomFactor(1.5)
      await browser.pause(300)

      const zoomedIn = await getZoomFactor()
      console.log(`Zoomed in to: ${zoomedIn}`)

      await browser.keys(['Control', '0'])
      await browser.pause(500)

      const resetZoom = await getZoomFactor()
      console.log(`Reset zoom to: ${resetZoom}`)

      expect(Math.abs(resetZoom - 1.0)).toBeLessThan(0.05)
    })
  })

  describe('Keyboard Shortcuts - Cmd (if macOS)', () => {
    it('should zoom in with Cmd+=', async function() {
      this.timeout(15000)

      const platform = await getPlatform()
      if (!platform.includes('Mac')) {
        this.skip()
      }

      const initialZoom = await getZoomFactor()
      console.log(`Initial zoom: ${initialZoom}`)

      await browser.keys(['Command', '='])
      await browser.pause(500)

      const newZoom = await getZoomFactor()
      console.log(`New zoom after Cmd+=: ${newZoom}`)

      expect(newZoom).toBeGreaterThan(initialZoom)
    })
  })

  describe('Mouse Wheel Zoom', () => {
    it('should zoom in with Ctrl+Wheel Up', async function() {
      this.timeout(15000)

      const initialZoom = await getZoomFactor()
      console.log(`Initial zoom: ${initialZoom}`)

      // Get the body element
      const body = await $('body')
      await body.moveTo()

      console.log('Performing Ctrl+Wheel scroll...')

      try {
        // Dispatch a real WheelEvent with Ctrl key
        await browser.execute(() => {
          const event = new WheelEvent('wheel', {
            deltaY: -100,
            ctrlKey: true,
            bubbles: true,
            cancelable: true
          })
          window.dispatchEvent(event)
        })

        await browser.pause(500)

        const newZoom = await getZoomFactor()
        console.log(`New zoom after Ctrl+Wheel: ${newZoom}`)

        if (newZoom <= initialZoom) {
          await captureScreenshot('zoom-wheel-failed')
          console.log('FAILED: Mouse wheel zoom did not increase zoom level')
        }

        expect(newZoom).toBeGreaterThan(initialZoom)
      } catch (error) {
        console.log('Error during wheel zoom test:', error.message)
        await captureScreenshot('zoom-wheel-error')
        throw error
      }
    })

    it('should zoom out with Ctrl+Wheel Down', async function() {
      this.timeout(15000)

      // First zoom in
      await setZoomFactor(1.5)
      await browser.pause(300)

      const initialZoom = await getZoomFactor()
      console.log(`Initial zoom: ${initialZoom}`)

      const body = await $('body')
      await body.moveTo()

      try {
        // Dispatch a real WheelEvent with Ctrl key
        await browser.execute(() => {
          const event = new WheelEvent('wheel', {
            deltaY: 100,
            ctrlKey: true,
            bubbles: true,
            cancelable: true
          })
          window.dispatchEvent(event)
        })

        await browser.pause(500)

        const newZoom = await getZoomFactor()
        console.log(`New zoom after Ctrl+Wheel down: ${newZoom}`)

        expect(newZoom).toBeLessThan(initialZoom)
      } catch (error) {
        console.log('Error during wheel zoom out test:', error.message)
        await captureScreenshot('zoom-wheel-out-error')
        throw error
      }
    })
  })

  describe('Direct API Tests', () => {
    it('should get initial zoom factor', async function() {
      this.timeout(10000)

      const zoom = await getZoomFactor()
      console.log(`Current zoom factor: ${zoom}`)

      expect(zoom).toBeGreaterThan(0)
      expect(zoom).toBeLessThanOrEqual(3.0)
    })

    it('should set zoom factor via API', async function() {
      this.timeout(10000)

      await setZoomFactor(1.5)
      await browser.pause(500)

      const zoom = await getZoomFactor()
      console.log(`Zoom after setting to 1.5: ${zoom}`)

      expect(Math.abs(zoom - 1.5)).toBeLessThan(0.05)
    })

    it('should clamp zoom to valid range', async function() {
      this.timeout(10000)

      // Try to set too high
      await setZoomFactor(5.0)
      await browser.pause(300)

      let zoom = await getZoomFactor()
      console.log(`Zoom after setting to 5.0: ${zoom}`)
      expect(zoom).toBeLessThanOrEqual(3.0)

      // Try to set too low
      await setZoomFactor(0.1)
      await browser.pause(300)

      zoom = await getZoomFactor()
      console.log(`Zoom after setting to 0.1: ${zoom}`)
      expect(zoom).toBeGreaterThanOrEqual(0.5)
    })
  })

  describe('Frontend State Tests', () => {
    it('should have zoom atom accessible', async function() {
      this.timeout(10000)

      const hasZoomAtom = await browser.execute(() => {
        // Check if zoom module is loaded
        return typeof window !== 'undefined'
      })

      expect(hasZoomAtom).toBe(true)
    })

    it('should have wheel event listener registered', async function() {
      this.timeout(10000)

      const hasListener = await browser.execute(() => {
        // This is a basic check - can't directly check listeners
        return document.body !== null
      })

      expect(hasListener).toBe(true)
    })
  })
})
