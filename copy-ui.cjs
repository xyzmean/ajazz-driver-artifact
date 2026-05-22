/**
 * Copy the generated UI (repo-root ./app) into ./app-ui so electron-packager
 * bundles a self-contained app. Run after the artifact pipeline has produced
 * ./app. Cross-platform (uses fs.cpSync, Node 16.7+).
 */
const fs = require("fs");
const path = require("path");

const src = path.resolve(__dirname, "app");
const dest = path.resolve(__dirname, "app-ui");

if (!fs.existsSync(path.join(src, "index.html"))) {
  console.error(`UI source not found at ${src}. Provide the scraped UI snapshot at ./app first.`);
  process.exit(1);
}

fs.rmSync(dest, { recursive: true, force: true });
fs.cpSync(src, dest, { recursive: true });
console.log(`Copied UI: ${src} -> ${dest}`);
