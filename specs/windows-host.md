# Windows host

Пользователь: иконка-монитор в трее, ЛКМ = toggle, ПКМ = меню (вкл/выкл, стиль, Настройки с языком, О программе, выход). Повторный запуск не плодит процесс — шлёт toggle уже живущему.

## Процесс

DPI: `PER_MONITOR_AWARE_V2` до окон.

Mutex `Local\BlackoutCinema.single`. Уже есть — `FindWindowW(BlackoutHost)` + `WM_EXTERNAL_TOGGLE`, выход 0.

Невидимый host (`BlackoutHost`, `WS_POPUP` + toolwindow): очередь, хоткеи, трей, таймеры. Оверлеи — отдельные `BlackoutOverlay`. Попап хоткея — `BlackoutBind`. Подтверждение зачистки — `BlackoutWipe`, не `bind_proc`. Скрытое 0×0 окно не может стать foreground — перед `SetForegroundWindow` host паркуют 1×1 за экраном.

Не rebuild оверлеев на `WM_SETTINGCHANGE` (обои, тема) — только ломает роли/layered. Топология: `WM_DISPLAYCHANGE` → таймер 400 мс → `rebuild_overlays` + `restore_after_hotplug`.

`TIMER_REASSERT` 500 мс, пока не Idle: вернуть topmost (панель задач Win11). Скрытое кино не трогать. `HWND_TOPMOST` на уже topmost окне z-order не двигает — restack через `HWND_TOP` (`restack_topmost`). После `ShowPick` / `ShowLocked` — `TIMER_ZORDER_KICK` (~10 мс × несколько раз): Explorer дописывает панель после нашего `WM_TRAY`.

## Трей

`NOTIFYICON_VERSION_4`: событие в **LOWORD** `lparam`. Один физический клик даёт и `NIN_SELECT`, и `WM_LBUTTONUP` — debounce ~350 мс, иначе Pick сразу снимется. Hover-подсказка — `szTip` «Blackout»; без `NIF_SHOWTIP` VERSION_4 её глушит.

ЛКМ по трею отдаёт foreground панели задач. Оверлеи `NOACTIVATE`, поэтому рамка на кадр уходит за панель. `tray_toggle` забирает foreground на host (как меню) и сразу restack; хоткей этот путь не трогает — плеер не паузить.

Иконка рисуется в `icon::draw_monitor_icon` (контур ~4:3, пустой центр, зелёная рамка, ножка) — и в трее, и в `.ico` через `build.rs`. Ширина корпуса той же чётности, что холст. Центр ножки — `mid_l`/`mid_r` инклюзивной рамки (при чётной ширине два пикселя, не «сдвиг на глаз»).

ICO: 16/20/24/32/40/48/64 — 32-bit DIB; 256 — PNG (Vista+). Windows масштабирует вниз, не вверх. Подписи Authenticode нет. VERSIONINFO: Product/FileDescription Blackout, Company/Copyright Alex Shink.

Первые ~400 мс клики по трею игнор (`tray_ready_at`) — оболочка шлёт ложный select.

## Оверлеи

`EnumDisplayMonitors`, сортировка left/top. Стиль окна: `WS_POPUP` + `TOPMOST | TOOLWINDOW | NOACTIVATE`. Создание скрытым, показ только из `set_role`.

Id = имя устройства (`szDevice`). Курсор → `MonitorFromPoint` + то же имя.

Виртуальные столы: пелена держится на всех. `TOPMOST | TOOLWINDOW` без кнопки на панели DWM не привязывает к одному столу. Не пинить через `IVirtualDesktopManager`.

Автозагрузка: галочка в **Настройки**, по умолчанию выкл. Источник правды — ОС, не `config.toml`. Windows: `HKCU\...\Run` + `StartupApproved\Run` (иначе Параметры → Автозагрузка глушит запись). Путь — текущий exe в кавычках. Сбой — тост. Не стартовать второй процесс при включении галочки.

Язык: **Настройки → Язык**, галочки как у стиля. Строки меню/тостов/попапов — `config.ui_lang()`. Подробности — [ui-language.md](ui-language.md).

Зачистка: [data-wipe.md](data-wipe.md). Перед попапом — `HideAll`. Сбой — тост, попап не закрывать.

**О программе**: отдельное окно `BlackoutAbout`, тот же вид. Имя, версия, автор, MIT, ссылка на GitHub (клик по ссылке или кнопке). **Закрыть (Esc)**. Пока открыт wipe/bind — about не открывать, фокус на уже открытом.

## Файлы

| Путь | Зачем |
|------|--------|
| `src/platform/windows.rs` | всё Win32 |
| `src/platform/windows.rs` `run_inner` | mutex, классы, цикл |
| `host_proc` | хоткей, трей, hotplug, apply pending |
| `tray_toggle` / `restack_topmost` / `kick_zorder` | z-order после ЛКМ по трею |
| `overlay_proc` | мышь, paint, DPI |
| `rebuild_overlays` / `monitor_enum` | топология |
| `add_tray` / `create_tray_icon` | трей |
| `src/icon.rs` / `src/icon_ico.rs` / `build.rs` | рисунок, ICO (DIB+PNG), VERSIONINFO |
| `src/about.rs` / `BlackoutAbout` | попап «О программе» |
| `src/autostart.rs` | Run / StartupApproved |
| `src/purge.rs` | wipe, текст попапа |
| `src/config.rs` | roaming AppData `\blackout\config.toml` (Known Folder, не env вслепую) |

## Не сделано

- Reassert не гарантирует победу над любой панелью / полноэкранным DirectX.

## Рядом

Роли окон — [veil-overlays.md](veil-overlays.md). Хоткей — [hotkey-bind.md](hotkey-bind.md). Режимы — [cinema-modes.md](cinema-modes.md).
