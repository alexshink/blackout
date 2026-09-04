# macOS panels

Пользователь: то же кино, трей — иконка-монитор (тот же рисунок, что на Windows). На этой машине не гонялось.

## Процесс

`NSApplicationActivationPolicy::Accessory` — без иконки в Dock. `OnceLock` — повторный `run` в процессе игнор (отдельного IPC как на Windows нет).

Оверлей — subclass `NSPanel`, **не** обычный `NSWindow`: `canBecomeKey/Main = false`, borderless + `NonactivatingPanel`, floating, уровень screensaver, мышь не ignore. Collection: все Spaces, fullscreen auxiliary, stationary, не в цикле Cmd-Tab.

Id = `NSScreenNumber`. Курсор — `NSEvent::mouseLocation` против `frame` панелей.

`listen_screens` пустой: смена мониторов подхватится при следующем toggle / явном rebuild, не сразу.

## Frost / кандидат

Veil Frost: `NSVisualEffectView` под контентом, material FullScreenUI. Снятие — `removeFromSuperview`. Tint — та же чёрная заливка с альфой, без effect view. Кандидат и Solid — заливка в `OverlayView::drawRect` (рамка обводкой, центр clear).

## Хоткей и трей

Carbon `RegisterEventHotKey`, сигнатура `BLKO`. Toggle id 1; pick-клавиши id 2–5 и 10–18 без модификаторов, пока `Pick`.

Статус-бар: иконка из `icon_png::PNG_32` (18 pt), не `●`. Меню пересобирается (`rebuild_status_menu`): Включить/Выключить, стили с галочками, **Настройки** (хоткей, автозагрузка, Язык, Удалить данные…) / **О программе** / Выход. Tooltip — «Blackout». ЛКМ по иконке сам по себе toggle не делает — только меню (в отличие от Windows). Язык — [ui-language.md](ui-language.md).

Иконка в Finder / Login Items — только у `.app`. `cargo build --release` на Mac + `scripts/macos-app.sh` → `target/release/Blackout.app` (`LSUIElement`, `AppIcon.icns` из `build.rs`). Голый бинарник в Finder серый; трей всё равно со своим рисунком.

О программе: тот же тёмный попап — имя, версия, автор, MIT, ссылка на GitHub. **GitHub** / **Закрыть (Esc)**. Пока bind/wipe — about не открывать.

Автозагрузка: галочка в Настройках, по умолчанию выкл. `~/Library/LaunchAgents/local.blackout.plist`, `RunAtLoad`, сессия Aqua. Только файл, без `launchctl bootstrap` / `bootout` — не плодить процесс и не убивать текущий. Источник правды — наличие plist, не конфиг.

Попап bind — key window, рисуется вручную. Сохранить слева, Отмена справа; хиттест по Y/X. Зачистка — отдельный `WipePanel` (уровень ~1000, не screensaver), wipe **до** `terminate`. Подробности — [data-wipe.md](data-wipe.md).

Конфиг: `~/Library/Application Support/blackout/config.toml`. `Alt` в строке = Option, `Super` = Cmd.

## Файлы

| Путь | Зачем |
|------|--------|
| `src/platform/macos.rs` | весь бэкенд |
| `make_panel` / `present` / `apply_frost` | окна |
| `install_hotkey` / `set_pick_keys` / `carbon_hotkey` | клавиши |
| `build_status` / `rebuild_status_menu` / `status_image` | трей, PNG-монитор |
| `scripts/macos-app.sh` | `.app` + icns |
| `begin_wipe` / `WipePanel` | подтверждение зачистки |
| `begin_about` / `AboutPanel` | О программе |
| `src/autostart.rs` | LaunchAgent |
| `src/purge.rs` | wipe |
| `src/hotkey.rs` | `mac_vk` / `mac_mods` / `key_from_mac_vk` |

## Не сделано

- Не проверено на железе.
- Hotplug не живой.
- Нет single-instance на второй запуск exe.
- `.app` не собирается сам из `cargo build` — нужен `scripts/macos-app.sh` на Mac.

## Дальше

- Живой listen `didChangeScreenParametersNotification`.

## Рядом

Режимы — [cinema-modes.md](cinema-modes.md). Пелена — [veil-overlays.md](veil-overlays.md). Хоткей — [hotkey-bind.md](hotkey-bind.md).
