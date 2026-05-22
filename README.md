# ajazz-driver-artifact — трекер апстрим-артефакта

Следит за апстрим WebHID-приложением Ajazz (`ajazz.driveall.cn`), пакует его в
воспроизводимый снапшот и при изменениях публикует релиз, заводит issue и
открывает кросс-репные PR. Скрейп вынесен сюда из сборки Electron —
`ajazz-driver-electron` больше не парсит сайт, а потребляет готовый релиз отсюда.

## Состав

```
ajazz-driver-artifact/
├── build_offline.py        discovery-скрейпер для ajazz.driveall.cn: рекурсивный fixpoint-обход
│                           (index.html → assets → import("./…") → langs/img),
│                           новые чанки подхватываются автоматически
├── build_hub_offline.py    discovery-скрейпер для www.ajazz-hub.com (AK980 PRO)
├── artifact_manifest.py    build/diff манифеста sha256 по каждому файлу (поддерживает мульти-директории)
├── reverse_with_vertex.py  реверс протокола через Vertex AI (Gemini)
├── manifest.json           закоммиченный базовый манифест (эталон для diff, содержит префиксы app/ и app-hub/)
└── .github/workflows/track-artifact.yml   пайплайн (cron раз в 3 дня + ручной)
```

## Как работает пайплайн

Триггеры: cron `0 3 */3 * *` (раз в 3 дня), ручной запуск (с входом
`force_vertex`), push в `main`.

1. `build_offline.py` скрейпит DriveAll сайт в `./app`, а `build_hub_offline.py` скрейпит Hub сайт в `./app-hub`.
2. `artifact_manifest.py build app,app-hub manifest.new.json` считает sha256 каждого файла с префиксом папки.
3. `artifact_manifest.py diff` сравнивает с закоммиченным `manifest.json`.
4. **Если отличий нет** — пайплайн завершается.
5. **Если есть отличия:**
   - `./app` пакуется в `ajazz-ui-ггммдд-ччмм.zip`;
   - `./app-hub` пакуется в `ajazz-ui-hub-ггммдд-ччмм.zip`;
   - обновлённый `manifest.json` коммитится обратно в этот репо (`[skip ci]`);
   - публикуется **релиз** `artifact-ггммдд-ччмм` (оба zip-снапшота + `manifest.json`);
   - заводится **issue** (здесь же) со списком added/removed/modified файлов;
   - открывается **кросс-репный PR в `ajazz-driver-electron`**, бампающий
     `artifact.lock` — мерж пересобирает Electron-драйвер на этот снапшот;
   - открываются **кросс-репные PR в `ajazz-driver-reverse`** (таблица моделей и,
     при изменении ядра, протокол через Vertex).

## Что считается «артефактом»

Весь `./app` (index.html + assets + langs + кэш картинок). Манифест —
**пофайловый** sha256, поэтому issue показывает конкретные изменившиеся файлы,
а не просто «что-то поменялось». Агрегатный `hash` манифеста пинится в
`artifact.lock` репозитория `ajazz-driver-electron` для воспроизводимости.

## Локальный запуск

```bash
python build_offline.py                          # → ./app
python artifact_manifest.py build app new.json    # манифест снапшота
python artifact_manifest.py diff manifest.json new.json   # что изменилось
```

## Авто-обновление `ajazz-driver-reverse`

При изменении снапшота пайплайн открывает в `ajazz-driver-reverse` PR двумя путями:

1. **Таблица моделей** (детерминированно): `extract_models.py` против нового
   `layout-default` → PR `reverse-models-<tag>`.
2. **Протокол** (`core.ts`/`commands.ts`) — если изменилось ядро (`index-*.js`):
   `reverse_with_vertex.py` шлёт в **Vertex AI (Gemini)** старый бандл + старый TS
   + новый бандл, получает обновлённый протокол + отчёт → PR `reverse-protocol-<tag>`.
   Это **предложение**, требует ревью; typecheck CI его проверяет. Реверс
   минифицированного кода детерминированно невозможен — отсюда LLM.

## Секреты и переменные репозитория

- **Secret `CROSS_REPO_TOKEN`** — PAT для кросс-репных PR/пушей в
  `ajazz-driver-electron` и `ajazz-driver-reverse` (права Contents + Pull requests:
  read/write на обоих). Без него релиз и issue создаются, но кросс-репные PR
  пропускаются (с warning).
- **Secret `GCP_SA_KEY`** — JSON-ключ сервис-аккаунта (`roles/aiplatform.user`).
  Без него шаги Vertex пропускаются.
- **Variables** `GCP_PROJECT`, `GCP_REGION` (напр. `us-central1`),
  `VERTEX_MODEL` (напр. `gemini-2.5-flash` — реальный id Vertex).

## Связь с другими репозиториями

- **`ajazz-driver-electron`** — потребитель: качает релиз отсюда по `artifact.lock`
  (получает кросс-репный bump-PR).
- **`ajazz-driver-reverse`** — получает кросс-репные PR на таблицу моделей и
  (через Vertex) на протокол.
