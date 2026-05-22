/**
 * Injected into the existing UI's page (main world) by the Electron host after
 * load. Attaches our reverse-engineered protocol to `window.ReverseDriver` and
 * mounts a small floating panel that drives the real device THROUGH OUR CODE
 * (src/protocol), alongside the upstream UI.
 *
 * Bundled for the browser by esbuild → dist-web/inject.js (IIFE).
 */
import * as proto from "../protocol";

/** Open the first HID device the page already has permission for. */
async function connect(): Promise<HIDDevice | null> {
  const devices = await navigator.hid.getDevices();
  const device = devices[0];
  if (!device) return null;
  if (!device.opened) await device.open();
  return device;
}

const PANEL_ID = "reverse-driver-panel";

function mountPanel(): void {
  if (document.getElementById(PANEL_ID)) return;

  const panel = document.createElement("div");
  panel.id = PANEL_ID;
  panel.style.cssText = [
    "position:fixed", "right:16px", "bottom:16px", "z-index:2147483647",
    "width:320px", "max-height:60vh", "overflow:auto",
    "background:#11151c", "color:#cfe3ff", "border:1px solid #2b3a52",
    "border-radius:10px", "box-shadow:0 8px 24px rgba(0,0,0,.45)",
    "font:12px/1.45 ui-monospace,Menlo,Consolas,monospace", "padding:12px",
  ].join(";");

  panel.innerHTML = `
    <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
      <strong style="color:#7db1ff">Reverse Driver</strong>
      <span style="opacity:.6">наш протокол</span>
    </div>
    <button id="rd-read" style="width:100%;padding:7px;border:0;border-radius:6px;
      background:#2563eb;color:#fff;cursor:pointer;font:inherit">Подключить и прочитать устройство</button>
    <pre id="rd-out" style="white-space:pre-wrap;margin:10px 0 0;color:#9fe6c0">готов…</pre>`;

  document.body.appendChild(panel);

  const out = document.getElementById("rd-out") as HTMLElement;
  const btn = document.getElementById("rd-read") as HTMLButtonElement;

  btn.onclick = async () => {
    btn.disabled = true;
    out.textContent = "чтение через наш протокол…";
    try {
      const device = await connect();
      if (!device) {
        out.textContent = "Устройство не найдено (подключите клавиатуру по USB).";
        return;
      }
      const info = await proto.getDeviceInfo(device);
      const model = proto.findModel(info.vid, info.pid);
      out.textContent = JSON.stringify(
        {
          model: model?.name ?? "(неизвестная модель)",
          firmware: info.version,
          vid: "0x" + info.vid.toString(16),
          pid: "0x" + info.pid.toString(16),
          battery: info.batteryLevel,
          profile: info.currentProfile,
          workMode: info.workMode,
        },
        null,
        2,
      );
    } catch (e) {
      out.textContent = "Ошибка: " + String(e);
    } finally {
      btn.disabled = false;
    }
  };
}

// Expose the full protocol surface for ad-hoc use from DevTools.
(window as unknown as { ReverseDriver: unknown }).ReverseDriver = { ...proto, connect, mountPanel };

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", mountPanel);
} else {
  mountPanel();
}
console.log("[ReverseDriver] injected — protocol attached to window.ReverseDriver");
