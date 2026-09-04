# Veil overlays

Пользователь видит: кино-экран чистый, остальные — чёрная пелена или матовое стекло; в выборе у кандидата прозрачный центр и зелёная рамка `#3DDC97`.

Мониторы **не** `Display Off` / не отстыковка: ОС сочтёт дисплей пропавшим, окна прыгнут. Пелена — окно на `rcMonitor` (весь экран, включая панель).

## Роли окна

Один оверлей на монитор, id стабилен в пределах текущей топологии.

| Роль | Когда | Вид |
|------|--------|-----|
| Hidden | `Idle`; кино в `Locked` | окно спрятано, не topmost-reassert |
| Candidate | `Pick`, выбранный id | рамка, центр почти прозрачный, клики живые |
| Veil | остальные в `Pick`/`Locked` | Solid — глухой чёрный; Tint — чёрный с альфой, без blur; Frost — тонировка + blur, если ОС умеет |

`WS_EX_TRANSPARENT` / click-through не ставить: клик по пелене должен доходить до нас (lock / dismiss).

Оверлей не активируется (`NOACTIVATE`, `MA_NOACTIVATE`, `NonactivatingPanel`): фокус плеера не крадём.

## Стили (`Style`)

`solid` (дефолт) / `tint` / `frost`. Общий `FROST_ALPHA` в `config.rs` — непрозрачность Tint и Frost, не «лёгкий туман».

Frost без композитора (старый Windows, GNOME/XFCE без KWin) — просто тёмная полупрозрачность. Это ожидаемо.

HDR: SDR-чёрный может выглядеть серым — не баг логики.

Exclusive fullscreen / часть DRM рисуют мимо композитора: на кино нашего окна нет, на остальных пелена может не накрыть такой плеер.

## Windows — поверхность (ломать нельзя)

`UpdateLayeredWindow` (кандидат) и `SetLayeredWindowAttributes` + acrylic (Frost veil) **взаимоисключающие**. Перед сменой роли — `reset_window_surface`: снять accent / `DWMSBT_NONE` и бит layered. Иначе Frost залипает, а Solid становится живым блюром.

Показ: `SW_SHOWNOACTIVATE`. Геометрия только `rcMonitor`, не `rcWork`.  
`reassert_topmost` (таймер) — `SWP_NOACTIVATE` **без** `SWP_SHOWWINDOW`, скрытое кино не всплывает. Restack: `HWND_TOPMOST` + `HWND_TOP` (одного TOPMOST мало, если окно уже topmost — панель задач после клика по трею остаётся выше).  
На скрытом окне не звать place с `SWP_SHOWWINDOW`.  
Смена layered-бита — `SWP_NOZORDER`, иначе `ensure_layered` сбивает стек.

Кандидат: `UpdateLayeredWindow`, центр alpha 1 (не 0 — иначе дырка клика). Рамка в DIP, множится на DPI окна.

## macOS / X11

macOS: `NSPanel` borderless, уровень screensaver, не key/main. Frost — `NSVisualEffectView` BehindWindow, иначе чёрная/тонированная заливка в `drawRect`.

X11: override-redirect на геометрию CRTC RandR. Frost — `_NET_WM_WINDOW_OPACITY` + опционально `_KDE_NET_WM_BLUR_BEHIND_REGION`. Кандидат: 32-bit ARGB + XRender, центр alpha 1, рамка `#3DDC97`. Без ARGB — непрозрачный чёрный центр.

## Файлы

| Путь | Зачем |
|------|--------|
| `src/config.rs` | `Style`, `FROST_ALPHA` |
| `src/platform/windows.rs` | `Role`, `set_role`, `reset_window_surface`, `paint_candidate`, `set_frost_blur` |
| `src/platform/macos.rs` | `present`, `apply_frost`, `paint` |
| `src/platform/x11.rs` | `map_overlay`, `paint_window`, `try_kwin_blur` |

## Не сделано

- Подсветка не гаснет (нет DDC).
- Wayland.
- Windows 11: панель задач иногда над пеленой — лечится reassert, не всегда сразу.

## Дальше

- DDC-яркость на veil.
- Wayland.

## Рядом

Режимы — [cinema-modes.md](cinema-modes.md). Host/трей — [windows-host.md](windows-host.md).
