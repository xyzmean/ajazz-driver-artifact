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

## Связь с другими ветками

- **`main`** — потребитель: качает релиз по `artifact.lock` (см. PR от пайплайна).
- **`reverse`** — при изменении бандла модели/протокол могут требовать ре-экстракта
  (`extract_models.py`); об этом напоминает issue (метка `reverse`).
