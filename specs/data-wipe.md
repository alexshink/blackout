# Data wipe

Пользователь: трей → **Настройки** → **Удалить данные…**. Попап подтверждает, что сотрём наши данные и выйдем. Exe и ярлыки не трогаем.

## Меню

Windows и macOS: Включить / Стиль (снаружи) / **Настройки** (хоткей, автозагрузка, Язык, разделитель, Удалить данные…) / О программе / Выход. Текст попапа — `purge::dialog_body(lang)`.

Linux без трея: попапа нет, `purge::wipe` в UI не зовём.

## Попап

Отдельное окно, не бинд хоткея. Тот же вид: тёмный фон, зелёный контур, **Удалить (Enter)** слева / **Отмена (Esc)** справа.

Перед показом: `HideAll` (Idle), закрыть бинд если был, снять toggle-хоткей. Пока попап открыт — бинд не открывать.

Текст честный: автозагрузка только если запись **наша**; наш `config.toml` и пустая папка `blackout`; закрытие программы; бинарник и ярлыки не удаляем. Prefetch и индекс ОС не обещаем чистить.

Ошибка wipe — попап остаётся, текст ошибки + тост (Windows). Успех — teardown и выход. После wipe **не** `config.save()`.

## Безопасность

`src/purge.rs` → `autostart::remove_if_ours` → `config::remove_our_files`. `remove_dir_all` запрещён. Поиска `*blackout*` по диску нет.

| Можно | Нельзя |
|---|---|
| `config.toml` по `strict_config_path()` | cwd / `./blackout` |
| `remove_dir` папки `blackout`, если пустая | рекурсивная зачистка |
| автозагрузка, если команда = наш exe (кавычки, `\\?\`, на Windows без регистра) | ключ/plist/desktop по имени, если путь чужой |
| Linux: наши `~/.local/share/icons/hicolor/*/apps/blackout.png` | чужие иконки в hicolor |
| | сам бинарник, ярлыки, Prefetch, Program Files, папка LaunchAgents / autostart |

Корень профиля: Windows `SHGetKnownFolderPath(RoamingAppData)`, иначе абсолютный `APPDATA`. Unix — `getpwuid`, иначе абсолютный `HOME`; Linux ещё абсолютный `XDG_CONFIG_HOME`. Нет профиля — `wipe` ошибка, `load`/`save` не пишут в `.`.

Порядок: автозагрузка (пока жив `current_exe`), потом конфиг, потом выход.

## Файлы

| Путь | Зачем |
|------|--------|
| `src/purge.rs` | `wipe`, текст попапа |
| `src/config.rs` | `strict_config_path`, `remove_our_files` |
| `src/autostart.rs` | `remove_if_ours`, сверка пути |
| `src/platform/windows.rs` | `BlackoutWipe`, `begin_wipe` |
| `src/platform/macos.rs` | `WipePanel` / `WipeView` |

## Не сделано

- Linux UI / флаг `--purge`.
- Самоудаление exe.
- Пункт в «Программы и компоненты».

## Дальше

- Нет отдельного пункта плана.

## Рядом

Трей — [windows-host.md](windows-host.md), [macos-panels.md](macos-panels.md). Конфиг — [crate-layout.md](crate-layout.md). Хоткей — [hotkey-bind.md](hotkey-bind.md).
