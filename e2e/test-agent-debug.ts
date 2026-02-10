import { _electron as electron } from "playwright";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function sleep(ms: number) { return new Promise(r => setTimeout(r, ms)); }

async function main() {
    console.log("Starting test...");
    
    const app = await electron.launch({
        executablePath: path.join(__dirname, "..", "make", "win-unpacked", "AgentMux.exe"),
        timeout: 60000,
    });
    
    const window = await app.firstWindow();
    console.log("Got window");
    await sleep(6000);
    
    // Create terminal
    console.log("Creating terminal...");
    const termWidget = window.locator("text=terminal").first();
    await termWidget.click();
    await sleep(4000);
    
    // Check localStorage before cd
    let logs = await window.evaluate(() => localStorage.getItem("agent-debug"));
    console.log("BEFORE CD:", logs);
    
    // Change directory
    console.log("Changing directory...");
    await window.keyboard.type("cd C:\Windows", {delay: 50});
    await window.keyboard.press("Enter");
    await sleep(4000);
    
    // Check localStorage after cd
    logs = await window.evaluate(() => localStorage.getItem("agent-debug"));
    console.log("AFTER CD:", logs);
    
    await app.close();
    console.log("Done");
}

main().catch(e => { console.error("Error:", e); process.exit(1); });
