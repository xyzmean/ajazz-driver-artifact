# artifact — трекер апстрим-артефакта

Ветка **`artifact`**: следит за апстрим WebHID-приложением Ajazz
(`ajazz.driveall.cn`), пакует его в воспроизводимый снапшот и при изменениях
заводит релиз, issue и PR. Скрейп вынесен сюда из сборки `main` — `main` больше
не парсит сайт при сборке, а потребляет готовый релиз отсюда.

## Состав

```
artifact/
├── build_offline.py       discovery-скрейпер: рекурсивный fixpoint-обход
│                          (index.html → assets → import("./…") → langs/img),
│                          новые чанки подхватываются автоматически
├── artifact_manifest.py   build/diff манифеста sha256 по каждому файлу
├── manifest.json          закоммиченный базовый манифест (эталон для diff)
└── .github/workflows/artifact.yml   пайплайн (push + cron раз в 3 дня)
```

## Как работает пайплайн

Триггеры: push в `artifact`, cron `0 3 */3 * *` (раз в 3 дня), ручной запуск.

1. `build_offline.py` скрейпит сайт в `./app` (рекурсивно — ловит новые файлы).
2. `artifact_manifest.py build` считает sha256 каждого файла → `manifest.new.json`.
3. `artifact_manifest.py diff` сравнивает с закоммиченным `manifest.json`.
4. **Если отличий нет** — пайплайн завершается.
5. **Если есть отличия:**
   - `./app` пакуется в `ajazz-ui-ггммдд-ччмм.zip`;
   - обновлённый `manifest.json` коммитится обратно в `artifact` (`[skip ci]`);
   - публикуется **релиз** `artifact-ггммдд-ччмм` (zip снапшота + `manifest.json`);
   - заводится **issue** со списком added/removed/modified файлов и метками
     `artifact-update` / `reverse` / `main`;
   - открывается **авто-PR в `main`**, бампающий `artifact.lock`
     (тег релиза + хеш манифеста) — мерж пересобирает Electron-драйвер на
     ровно этот снапшот.

## Что считается «артефактом»

Весь `./app` (index.html + assets + langs + кэш картинок). Манифест —
**пофайловый** sha256, поэтому issue показывает конкретные изменившиеся файлы,
а не просто «что-то поменялось». Агрегатный `hash` манифеста пинится в
`artifact.lock` ветки `main` для воспроизводимости.

## Локальный запуск

```bash
python build_offline.py                          # → ./app
python artifact_manifest.py build app new.json    # манифест снапшота
python artifact_manifest.py diff manifest.json new.json   # что изменилось
```

## Авто-обновление ветки `reverse`

При изменении снапшота пайплайн обновляет `reverse` двумя путями:

1. **Таблица моделей** (детерминированно): `extract_models.py` против нового
   `layout-default` → авто-PR `reverse-models-<tag>`.
2. **Протокол** (`core.ts`/`commands.ts`) — если изменилось ядро (`index-*.js`):
   `reverse_with_vertex.py` шлёт в **Vertex AI (Gemini)** старый бандл + наш
   старый TS + новый бандл, получает обновлённый протокол + отчёт → авто-PR
   `reverse-protocol-<tag>`. Это **предложение**, требует ревью; typecheck CI его
   проверяет. Реверс минифицированного кода детерминированно невозможен — отсюда LLM.

### Настройка Vertex (секреты/переменные репо)

- Secret `GCP_SA_KEY` — JSON-ключ сервис-аккаунта с ролью `roles/aiplatform.user`.
- Variables `GCP_PROJECT`, `GCP_REGION` (напр. `us-central1`),
  `VERTEX_MODEL` (напр. `gemini-3.5-flash`).

Если `GCP_SA_KEY` не задан — шаги Vertex не сработают, но остальной пайплайн
(модели, main-bump PR) не падает.

## Связь с другими ветками

- **`main`** — потребитель: качает релиз по `artifact.lock` (см. PR от пайплайна).
- **`reverse`** — получает авто-PR на таблицу моделей и (через Vertex) на протокол;
  issue с меткой `reverse` остаётся сводкой изменений.
