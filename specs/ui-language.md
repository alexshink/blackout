# UI language

Пользователь: Windows/macOS — **Настройки → Язык → Русский / English** (галочка). Linux — ключ в `config.toml` или язык UI ОС.

## Резолв

`Config.language: Option<Language>`. Нет ключа — `Language::from_os()`: UI ОС `ru*` / `en*`, иначе русский. В файл пишется только после выбора в меню (на Linux — ручная правка). Смена стиля/хоткея язык не пинит (`skip_serializing_if`).

Детект: Windows `GetUserPreferredUILanguages` (язык UI, не регион дат); macOS `NSLocale.preferredLanguages`, иначе `LANG`; Linux `LANGUAGE` → `LC_ALL` → `LC_MESSAGES` → `LANG`.

Галочка в меню — на *резолвнутом* языке. Имена пунктов — эндонимы («Русский», «English»), не из каталога.

## Каталог

`src/i18n.rs`: `Language::ALL`, таблицы `Tr`. Новый язык — вариант enum + колонка `Tr`. Не переводим: Blackout, автор, MIT, URL, токены `Ctrl`/`Alt`/`Shift`/`Win`/`Cmd`/`Super`/`Enter`/`Esc`, ключи toml.

Интерполяция — методы `Language` (`{hk}`, `{path}`). Подсказка bind подставляет `Win` / `Cmd` / `Super` по ОС.

## Платформы

Windows: подменю в `show_tray_menu`, `CMD_LANG_BASE + index`. Смена — `save` + `InvalidateRect` открытых about/wipe/bind.

macOS: `rebuild_status_menu` при языке / стиле / хоткее / toggle. `statusLang:` + `tag`.

X11: меню нет. Строки попапа и stderr из `Tr`. Правка toml — после перезапуска.

## Файлы

| Путь | Зачем |
|------|--------|
| `src/i18n.rs` | enum, детект, `Tr` |
| `src/config.rs` | `language`, `ui_lang()` |
| `src/purge.rs` | `dialog_body(lang)` |
| `src/platform/windows.rs` | меню, тосты, попапы |
| `src/platform/macos.rs` | меню, панели |
| `src/platform/x11.rs` | bind, stderr |

## Не сделано

- Пункт «Как в системе».
- Локализация `.desktop` Comment / реестра автозагрузки.
- Шрифт X11 кроме ISO-8859-5 (ru/en влезают).

## Рядом

Конфиг — [crate-layout.md](crate-layout.md). Трей — [windows-host.md](windows-host.md), [macos-panels.md](macos-panels.md). Wipe — [data-wipe.md](data-wipe.md).
