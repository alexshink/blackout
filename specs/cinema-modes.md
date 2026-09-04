# Cinema modes

Пользователь: хоткей / трей — выбрать монитор, зафиксировать кино, снять пелену. Оверлей не активируется, чтобы плеер не паузился.

## Состояния (`Mode`)

`Idle` → `Pick` → `Locked` → `Idle`.

| Режим | Экраны | Как выйти |
|-------|--------|-----------|
| `Idle` | окон нет | toggle: хоткей, ЛКМ по трею, повторный exe (Windows) |
| `Pick` | кандидат + пелены | клик / Enter → `Locked`; toggle / Esc → `Idle` |
| `Locked` | кино скрыто, остальные пелена | toggle или клик по пелене → `Idle` |

Toggle из `Pick` и `Locked` всегда `dismiss` (`HideAll`). Из `Idle` — `enter_pick` по монитору под курсором; нет дисплея — `Action::None`.

## `Action` (платформа только это рисует)

`None` / `HideAll` / `ShowPick { candidate }` / `RefreshPick { candidate }` / `ShowLocked { cinema }`.

`ShowPick` и `RefreshPick` для окон одно и то же: кандидат + пелены. Разница только зачем логика послала (вход vs смена кандидата).

Смена стиля: в `Idle` только запомнить; в `Pick`/`Locked` — перерисовать текущую раскладку.

## Выбор без мыши (`PickKey`)

Только в `Pick`. Цифры `1`–`9` — индекс в списке дисплеев платформы (слева направо). Стрелки циклически. Enter — текущий кандидат, иначе первый в списке. Esc — снять всё.

После цифры или стрелки `follow_pointer = false`: ховер не двигает рамку, пока платформа не сообщит реальный сдвиг мыши (порог в платформе). Иначе курсор, оставшийся на старом экране, сразу сбросит клавиатурный выбор.

Клик в `Pick` фиксирует **тот** монитор, по которому кликнули (не обязательно кандидат). Клик в `Locked` снимает всё.

## Hotplug (`restore_after_hotplug`)

Список id пришёл заново. `Idle` — спрятать. `Pick` — тот же кандидат, если жив, иначе `enter_pick` первого. `Locked` — то же кино, если живо, иначе `dismiss`.

## Файлы

| Путь | Зачем |
|------|--------|
| `src/app.rs` | `Mode`, `PickKey`, `Action`, `Logic` |
| `src/platform/windows.rs` | `apply`, `handle_hotkey`, `pointer_moved_enough` |
| `src/platform/macos.rs` | `apply`, `carbon_hotkey`, порог в `pointer` |
| `src/platform/x11.rs` | `apply`, `handle_key` / `handle_button` / `handle_enter` |

Точки входа в логику: `on_toggle`, `on_pointer`, `on_click`, `on_pick_key`, `set_style`, `restore_after_hotplug`.

## Не сделано

- X11 после стрелок/цифр `follow_pointer` гасится, но мышиный порог не снимает его — ховер кандидата может не вернуться до нового toggle.
- Виртуальные столы Windows: пелена остаётся на столе, где открыли.

## Дальше

- Авто-диммер по фокусу плеера.

## Рядом

Окна — [veil-overlays.md](veil-overlays.md). Хоткей — [hotkey-bind.md](hotkey-bind.md).
