# Hotkey bind

Пользователь: глобальный хоткей включает/снимает режим. Смена — попап: превью через `+`, Сохранить (Enter) слева / Отмена (Esc) справа (подписи из `Tr`). Пока не Сохранить — старая комбинация.

## Формат

Одна строка в `config.toml`: `Ctrl+Alt+B`. Нужен хотя бы один модификатор.

| Токен | Смысл |
|-------|--------|
| `Ctrl` / `Control` | Control |
| `Alt` / `Option` | Alt; на Mac — Option |
| `Shift` | Shift |
| `Super` / `Win` / `Cmd` / `Meta` | Win / Cmd |
| `A`–`Z`, `0`–`9`, `F1`–`F12`, `Space` | клавиша |

Парсинг и VK/keysym — только `src/hotkey.rs`. Платформа не парсит строку сама.

Дефолт: Ctrl+Alt+B. Файл без ключа `hotkey` при загрузке переписывается (миграция старых конфигов).

Правка файла вручную — после рестарта. Смена из UI — сразу.

## Регистрация

Не кейлоггер: `RegisterHotKey` / Carbon `RegisterEventHotKey` / `XGrabKey`.

Windows: сначала с `MOD_NOREPEAT`; если отказ — без него. Занятость чужим приложением — `hotkey_ok = false`, тост, пункт трея «(занят)». Своя же комбинация при rebind — успех.

Windows apply **не** внутри `DestroyWindow` попапа: `commit_bind` кладёт `pending_hotkey`, `PostMessage(WM_APPLY_HOTKEY)`. Иначе регистрация срывается.

X11: grab на root во всех вариантах Lock/NumLock (`x11_mod_variants`).

В `Pick` дополнительно регистрируются голые Esc / Enter / стрелки / `1`–`9` (Windows, macOS). На X11 эти клавиши читаются из событий, пока оверлеи замаплены.

## Попап

На время захвата toggle снимается, чтобы новая комбинация не сразу сработала.

Модификаторы: Windows — `GetKeyState` для Ctrl/Alt/Shift; Win **только** если реально нажат (`bind_win` на keydown/keyup). Не `GetAsyncKeyState` — ловит залипший Win и помечает любую комбинацию занятой.

Превью живое, пока комбинация неполная. Как только draft валиден (модификатор + клавиша) — превью замирает, пока не отпустят все клавиши этой комбинации. Следующее нажатие после полного отпускания начинает ввод с нуля. Enter без модификаторов = Сохранить, если draft есть. Esc = отмена, вернуть старый хоткей. Кнопки рисуются вручную: зелёный контур, без заливки; Сохранить без draft неактивна.

Linux без трея: тот же попап по ПКМ по пелене; Enter/клик Сохранить, клик Отмена или Esc. Если стартовый grab не удался — попап сразу.

## Файлы

| Путь | Зачем |
|------|--------|
| `src/hotkey.rs` | `Hotkey`, parse/display, `win_*` / `mac_*` / `x11_*` |
| `src/config.rs` | хранение, путь файла |
| `src/autostart.rs` | галочка трея → Run / LaunchAgent / XDG `.desktop` |
| `src/platform/windows.rs` | `register_toggle_hotkey`, `begin_bind`, `apply_pending_hotkey` |
| `src/platform/macos.rs` | `install_hotkey`, `begin_bind`, `commit_bind` |
| `src/platform/x11.rs` | `grab_hotkey`, `begin_bind`, `finish_bind` |

## Не сделано

- Захват на X11 — раскладка US QWERTY (keycodes), не произвольный keymap.
- macOS: не все символы имеют `mac_vk` — регистрация тихо не встанет.
- Single-instance и тосты о занятом хоткее — только Windows.

## Дальше

- Нет отдельного пункта плана на хоткей.

## Рядом

Режимы — [cinema-modes.md](cinema-modes.md). Конфиг/стиль — [crate-layout.md](crate-layout.md). Трей Windows — [windows-host.md](windows-host.md).
