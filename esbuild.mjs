// Bundle the browser-side injection script (protocol + dev panel) into a single
// IIFE that the Electron host injects into the existing UI page.
import { build } from "esbuild";

await build({
  entryPoints: ["src/web/inject.ts"],
  bundle: true,
  format: "iife",
  platform: "browser",
  target: "es2020",
  outfile: "dist-web/inject.js",
  logLevel: "info",
});
