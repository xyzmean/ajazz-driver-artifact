# ⌨️ AJAZZ Offline Desktop Driver / Офлайн-драйвер AJAZZ (Electron)

[English](#english) | [Русский](#русский)

---

# English

A **100% offline, standalone desktop configuration utility** for the **Ajazz
AK980 MAX** and other supported Ajazz keyboards. It wraps the original WebHID UI
into a native Windows app built with **Electron**, enabling local customization
without an internet connection.

This is the **`main`** (production) branch — the proven wrapper. It **no longer
scrapes the vendor site at build time**: the UI snapshot is produced by the
[`artifact`](#repository-branches) pipeline and consumed here as a pinned
release.

## Repository branches

| Branch | Purpose |
|--------|---------|
| **`main`** | Production Electron wrapper. Builds from a pinned UI snapshot (`artifact.lock`); no scraping. |
| **`artifact`** | Upstream tracker: scrapes the vendor site (push + every 3 days), diffs by per-file checksum, publishes a snapshot release, opens an issue and a PR bumping `artifact.lock` here. |
| **`reverse`** | Our reverse-engineered HID driver (typed protocol layer + model table). |
| **`dev-tauri`** | Standalone Tauri (Rust + hidapi) + Vue 3 driver, pinned to a `reverse` commit. |

## How the snapshot reaches `main`

`artifact.lock` pins the exact UI snapshot release this branch builds against:

```json
{ "release_tag": "artifact-YYMMDD-HHMM", "manifest_hash": "…" }
```

The `artifact` pipeline opens an auto-PR bumping this file when upstream changes;
merging it triggers a rebuild against that exact snapshot. An empty `release_tag`
makes CI fall back to the latest `artifact-*` release.

## 🌟 Key features
- **100% offline**: all JS, translations, and keyboard images are served from local relative paths.
- **Zero-configuration WebHID**: the Electron backend auto-detects and pairs the USB keyboard — no browser prompts.
- **Adaptive scaling**: launches maximized with `zoomFactor: 0.85` so the UI fits laptop displays.
- **Reproducible CI**: builds from a checksum-pinned snapshot, not a live scrape — same input, same output.

## ⌨️ Supported keyboards

AK980 (`MAX`/`PRO`/`PRO 2.4G`/`V2 PRO`), AK820 (`AK820`/`AK820MAX`/`AK820 MAX
Lightles`/`820PRO`), AK680 (`MAX`/`V2`), `AK870MC`, `ALUX75 PRO`, `AK029`,
`AK039`, `MJ84+`, `QS87`, `CSOL Keyboard`. The full set tracks the model table
in the `reverse` branch (`models.json`, 42 entries).

## 🤖 CI/CD (GitHub Actions)
1. **Triggers**: every push to `main` (including merged `artifact.lock` bumps) and manual **Run workflow**. The periodic upstream check lives in the `artifact` branch, not here.
2. **Build**: resolves the pinned snapshot → downloads that release → unpacks `app/` → `electron-packager`.
3. **Result**: a ZIP with `AJAZZ Local Driver.exe`, attached to a release tagged `YYMMDD-HHMM`.

## 🚀 Run locally

Requires **Node.js 18+**. Provide the UI snapshot at `./app` (download a
`ajazz-ui-*.zip` from an `artifact-*` release and unzip into `app/`, or run the
`artifact` branch's `build_offline.py`). Then:

```bash
npm install
npm start              # production
npm run start:debug    # with DevTools
npm run build          # package standalone .exe into dist/
```

> [!IMPORTANT]
> When changing keymaps, Rapid Trigger, and lighting, connect the keyboard
> **strictly via USB cable** (not Bluetooth or 2.4 GHz). WebHID works only over a
> direct wire.

---

# Русский

**100% автономная локальная программа настройки** для **Ajazz AK980 MAX** и
других поддерживаемых клавиатур Ajazz. Оборачивает оригинальный WebHID-интерфейс
в нативное Windows-приложение на **Electron** — настройка без интернета.

Это ветка **`main`** (прод) — проверенная обёртка. Она **больше не парсит сайт
производителя при сборке**: UI-снапшот готовит пайплайн ветки
[`artifact`](#ветки-репозитория), а сюда он попадает как запиненный релиз.

## Ветки репозитория

| Ветка | Назначение |
|-------|------------|
| **`main`** | Прод Electron-обёртка. Собирается из запиненного снапшота (`artifact.lock`), без скрейпа. |
| **`artifact`** | Трекер апстрима: скрейпит сайт (push + раз в 3 дня), сравнивает по пофайловым суммам, публикует релиз-снапшот, заводит issue и PR с бампом `artifact.lock`. |
| **`reverse`** | Наш реверс-инжиниринг HID-драйвера (типизированный протокол + таблица моделей). |
| **`dev-tauri`** | Автономный драйвер на Tauri (Rust + hidapi) + Vue 3, запиненный на коммит `reverse`. |

## Как снапшот попадает в `main`

`artifact.lock` пинит конкретный релиз-снапшот, на котором собирается ветка:

```json
{ "release_tag": "artifact-ггммдд-ччмм", "manifest_hash": "…" }
```

Пайплайн `artifact` при изменении апстрима открывает авто-PR с бампом этого
файла; мерж пересобирает драйвер ровно на этот снапшот. Пустой `release_tag` —
CI берёт последний релиз `artifact-*`.

## 🌟 Ключевые особенности
- **Полный офлайн**: все JS, переводы и изображения раскладок отдаются с локальных относительных путей.
- **WebHID без настройки**: Electron-бэкенд сам находит и подключает USB-клавиатуру, минуя браузерные окна.
- **Адаптивный интерфейс**: запуск развёрнутым, `zoomFactor: 0.85` для экранов ноутбуков.
- **Воспроизводимая сборка**: из запиненного по контрольной сумме снапшота, а не из живого скрейпа.

## ⌨️ Поддерживаемые клавиатуры

AK980 (`MAX`/`PRO`/`PRO 2.4G`/`V2 PRO`), AK820 (`AK820`/`AK820MAX`/`AK820 MAX
Lightles`/`820PRO`), AK680 (`MAX`/`V2`), `AK870MC`, `ALUX75 PRO`, `AK029`,
`AK039`, `MJ84+`, `QS87`, `CSOL Keyboard`. Полный список соответствует таблице
моделей в ветке `reverse` (`models.json`, 42 записи).

## 🤖 Сборка (GitHub Actions)
1. **Триггеры**: каждый push в `main` (включая мерж бампа `artifact.lock`) и ручной запуск. Периодическая проверка апстрима — в ветке `artifact`, не здесь.
2. **Сборка**: резолвит запиненный снапшот → качает релиз → распаковывает `app/` → `electron-packager`.
3. **Результат**: ZIP с `AJAZZ Local Driver.exe` в релизе с тегом `ггммдд-ччмм`.

## 🚀 Локальный запуск

Нужен **Node.js 18+**. Положите UI-снапшот в `./app` (скачайте `ajazz-ui-*.zip`
из релиза `artifact-*` и распакуйте в `app/`, либо запустите `build_offline.py`
из ветки `artifact`). Затем:

```bash
npm install
npm start              # рабочий режим
npm run start:debug    # с DevTools
npm run build          # портативный .exe в dist/
```

> [!IMPORTANT]
> Для смены раскладки, Rapid Trigger и подсветки подключайте клавиатуру
> **строго по USB-кабелю** (не Bluetooth и не 2.4 ГГц). WebHID работает только по
> прямому проводу.
