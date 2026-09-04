# X11 session

Пользователь: тот же хоткей и пелена, **без трея**. Смена хоткея — ПКМ по пелене (Enter/Сохранить, Esc/Отмена) или правка `~/.config/blackout/config.toml`. Язык UI — ключ `language` или язык ОС; [ui-language.md](ui-language.md).

## Сессия

Если `XDG_SESSION_TYPE=wayland` и нет `DISPLAY` — выход с ошибкой (нужен X11 или XWayland). Wayland-натива нет.

Цикл: `wait_for_event`. Старт пишет в stderr хоткей и путь конфига (строки из `Tr`).

## Окна

RandR: подключённые output с живым CRTC, id `OUTPUT-{i}`, сортировка x/y. Нет RandR — один `SCREEN-0` на корень.

Override-redirect на геометрию CRTC, события: кнопка, enter, motion, expose, visibility. Visibility → снова `ABOVE`.

ПКМ (button 3) на любом оверлее — bind. Попап — в центре того output, где курсор (ПКМ или `query_pointer`), не по bounding box всего виртуального экрана. В попапе ЛКМ только по Сохранить / Отмена. ЛКМ по пелене — `on_click`. Подписи попапа — битмап (ISO-8859-5), не `ImageText` и не системный XLFD: на Mint/современных сессиях core fonts часто пустые.

Pick-клавиши: `XGrabKey` на root, пока `Pick` (Esc / Enter / стрелки / `1`–`9`, четыре маски Lock/NumLock). Оверлей без фокуса — без grab KeyPress не приходит. Снимать grab в Idle, Locked и на время bind. Коды US: Esc 9, Enter 36, Left 113, Right 114, `1`–`9` = 10–18.

Кандидат: окно на весь CRTC (клик наш). Вид — XRender ARGB, центр alpha 1 как на Windows, рамка непрозрачная. Нет ARGB — чёрное поле + рамка. SHAPE-дырку не использовать: Cinnamon/Mutter берут bounding и как input, клик проходит насквозь.

## Хоткей

`XGrabKey` на root, четыре маски Lock/NumLock. Не встал при старте — сразу `begin_bind`. Grab клавиатуры на попап, чтобы ловить клавиши без фокуса WM.

Захват новой комбинации — keycodes US QWERTY, в конфиг пишется буква.

## Файлы

| Путь | Зачем |
|------|--------|
| `src/platform/x11.rs` | весь бэкенд |
| `rebuild` | RandR → окна |
| `grab_hotkey` / `begin_bind` / `finish_bind` | хоткей |
| `handle` / `apply` / `map_overlay` | события и роли |
| `src/config.rs` | `XDG_CONFIG_HOME` / `~/.config/blackout/config.toml` |
| `src/autostart.rs` | `~/.config/autostart/blackout.desktop`, `Icon=` + PNG в `~/.local/share/icons/hicolor/` |

## Не сделано

- Трей / StatusNotifier. Автозагрузка и зачистка из UI недоступны; вручную: `~/.config/blackout/` и `~/.config/autostart/blackout.desktop`. `purge::wipe` есть, флага `--purge` нет.
- Живой hotplug: комментарий про RandR есть, rebuild по notify нет.
- После клавиатурного выбора ховер может не вернуть кандидата (`follow_pointer`, см. cinema-modes).
- Нет single-instance.

## Дальше

- Wayland.
- Трей на Linux не планировался как must.

## Рядом

Режимы — [cinema-modes.md](cinema-modes.md). Пелена — [veil-overlays.md](veil-overlays.md). Хоткей — [hotkey-bind.md](hotkey-bind.md).
