# Crate layout

Один бинарник `blackout`: пелена на все мониторы кроме выбранного. Мониторы не отключаются. Без UI-тулкита, Electron, Qt.

## Сборка

`cargo build --release` на той ОС, которую тестируете. Кросс-компиляция чужой платформы не используется: `objc2` / `x11rb` / `windows` подключаются только своим `cfg`.

Выход: `target/release/blackout` (Windows — `blackout.exe`). Release: LTO, один codegen-unit, `panic = abort`, strip. На Windows в release `#![windows_subsystem = "windows"]` — без консоли.

`.cargo/config.toml`: `target-dir = "target"`; на Windows MSVC — `+crt-static`, чтобы `blackout.exe` не требовал `VCRUNTIME140.dll`.

## Кто кого зовёт

`src/main.rs` → `platform::run()`. Ошибка — `eprintln` и код 1.

`src/platform/mod.rs` выбирает ровно один бэкенд:

| cfg | Модуль | `run` |
|-----|--------|--------|
| `windows` | `platform/windows.rs` | Win32 message loop |
| `target_os = "macos"` | `platform/macos.rs` | `NSApplication::run` |
| unix, не macOS | `platform/x11.rs` | X11 event loop |

Общее не знает HWND / NSPanel / Window:

| Модуль | Отвечает |
|--------|----------|
| `src/app.rs` | Режимы, `Action` для платформы |
| `src/config.rs` | `config.toml`, `Style`, `language` (`Option`, нет ключа = UI ОС), `FROST_ALPHA`. Путь — профиль ОС, не cwd. |
| `src/i18n.rs` | `Language` ru/en, `Tr`, детект UI ОС. |
| `src/hotkey.rs` | Строка комбинации ↔ VK / keysym |
| `src/autostart.rs` | Галочка трея; HKCU Run / LaunchAgent / XDG autostart. Не в конфиге. |
| `src/purge.rs` | Зачистка наших путей; exe не трогает. |
| `src/about.rs` | Имя, автор, MIT, GitHub, версия из `CARGO_PKG_VERSION`. |
| `src/icon.rs` | Рисунок иконки-монитора (трей Windows). |
| `src/icon_ico.rs` | Сборка `.ico` / `.icns` / PNG при билде. |
| `src/icon_png.rs` | Вшитые PNG 32/48/256 (`OUT_DIR`), не Windows. |
| `build.rs` | PNG+ICNS всегда; Windows ещё ICO + VERSIONINFO. |
| `scripts/macos-app.sh` | На Mac: `Blackout.app` из release + `target/blackout.icns`. |
| `LICENSE` | MIT, Copyright 2026 Alex Shink. |

Платформа: окна, трей, регистрация хоткея, покраска, hotplug. Логику «какой монитор кино» не дублировать в трёх файлах — только `Logic` + `apply(Action)`.

## Зависимости

Всегда: `serde`, `toml`.  
Всегда в `build.rs`: `png`. Windows ещё `winresource`.  
Windows: crate `windows` 0.61 (GDI, DWM, shell, hotkey, DPI).  
macOS: `objc2` + AppKit/Foundation, Carbon линкуется вручную.  
Linux: `x11rb` + RandR + Render. Попап хоткея рисует текст через `embedded-graphics` (ISO-8859-5), без X core fonts.

## Не сделано

- Сборки под чужую ОС с этой машины нет.
- Тестов почти нет: roundtrip хоткея в `hotkey.rs`.

## Дальше

- Wayland-бэкенд (сейчас отказ, если нет `DISPLAY`).

## Рядом

Режимы — [cinema-modes.md](cinema-modes.md). Окна — [veil-overlays.md](veil-overlays.md). Продуктовый обзор — корневой README.
