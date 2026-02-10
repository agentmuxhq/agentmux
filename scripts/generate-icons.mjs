// Generate Tauri app icons from landing page SVG
import sharp from "sharp";
import { readFileSync, writeFileSync, mkdirSync } from "fs";
import { join } from "path";
import { execSync } from "child_process";

const svgPath = join(import.meta.dirname, "..", "landing", "logo.svg");
const iconsDir = join(import.meta.dirname, "..", "src-tauri", "icons");
const assetsDir = join(import.meta.dirname, "..", "assets");

// Read SVG and set explicit size for rendering
const svgBuffer = readFileSync(svgPath);

// Sizes needed for Tauri icons
const sizes = [
  { name: "32x32.png", size: 32 },
  { name: "128x128.png", size: 128 },
  { name: "128x128@2x.png", size: 256 },
  { name: "icon.png", size: 512 },
];

async function main() {
  // Generate PNGs at each size
  for (const { name, size } of sizes) {
    const outPath = join(iconsDir, name);
    await sharp(svgBuffer, { density: 400 })
      .resize(size, size)
      .png()
      .toFile(outPath);
    console.log(`Generated ${name} (${size}x${size})`);
  }

  // Generate ICO (Windows) - contains 16, 24, 32, 48, 64, 128, 256 sizes
  const icoSizes = [16, 24, 32, 48, 64, 128, 256];
  const icoBuffers = [];
  for (const size of icoSizes) {
    const buf = await sharp(svgBuffer, { density: 400 })
      .resize(size, size)
      .png()
      .toBuffer();
    icoBuffers.push({ size, buf });
  }

  // Build ICO file manually
  const icoBuffer = buildIco(icoBuffers);
  writeFileSync(join(iconsDir, "icon.ico"), icoBuffer);
  console.log(`Generated icon.ico (${icoSizes.length} sizes)`);

  // Copy SVG to assets
  const svgContent = readFileSync(svgPath, "utf8");
  writeFileSync(join(assetsDir, "agentmux-logo.svg"), svgContent);
  console.log("Copied logo SVG to assets/agentmux-logo.svg");

  // Generate appicon-windows.png
  await sharp(svgBuffer, { density: 400 })
    .resize(256, 256)
    .png()
    .toFile(join(assetsDir, "appicon-windows.png"));
  console.log("Generated assets/appicon-windows.png");

  console.log("\nDone! ICNS (macOS) needs to be generated on macOS or via iconutil.");
  console.log("For now, the ICO and PNGs are ready for Windows builds.");
}

// Build ICO file from PNG buffers
function buildIco(entries) {
  const headerSize = 6;
  const dirEntrySize = 16;
  const numImages = entries.length;

  let offset = headerSize + dirEntrySize * numImages;
  const dirEntries = [];
  const imageData = [];

  for (const { size, buf } of entries) {
    dirEntries.push({
      width: size >= 256 ? 0 : size,
      height: size >= 256 ? 0 : size,
      offset,
      size: buf.length,
    });
    imageData.push(buf);
    offset += buf.length;
  }

  const totalSize = offset;
  const ico = Buffer.alloc(totalSize);

  // ICO header
  ico.writeUInt16LE(0, 0); // reserved
  ico.writeUInt16LE(1, 2); // type: ICO
  ico.writeUInt16LE(numImages, 4); // count

  // Directory entries
  let pos = headerSize;
  for (let i = 0; i < numImages; i++) {
    const entry = dirEntries[i];
    ico.writeUInt8(entry.width, pos);
    ico.writeUInt8(entry.height, pos + 1);
    ico.writeUInt8(0, pos + 2); // color palette
    ico.writeUInt8(0, pos + 3); // reserved
    ico.writeUInt16LE(1, pos + 4); // color planes
    ico.writeUInt16LE(32, pos + 6); // bits per pixel
    ico.writeUInt32LE(entry.size, pos + 8); // data size
    ico.writeUInt32LE(entry.offset, pos + 12); // data offset
    pos += dirEntrySize;
  }

  // Image data
  for (const buf of imageData) {
    buf.copy(ico, pos);
    pos += buf.length;
  }

  return ico;
}

main().catch(console.error);
