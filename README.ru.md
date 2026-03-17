# Transmission Remote — Slint

Лёгкий нативный GUI-клиент для **демона Transmission**, написанный на **Rust + Slint**.  
Без GTK, без Qt — нативный рендеринг через Skia/OpenGL или Vulkan.

> **Разработано с помощью Claude (Anthropic AI).**

[English](README.md)

---

## Сравнение

| Функция | **transmission-remote-slint** | transmission-remote-gtk | transmission-qt | Transmission GTK 4.1 |
|---|---|---|---|---|
| Тип | Только remote | Только remote | Standalone + Remote | Standalone |
| Тулкит | Slint (Rust) | GTK 3 | Qt 5/6 | GTK 4 |
| Зависимость от GTK | ✅ Нет | ❌ GTK 3 | ❌ Qt libs | ❌ GTK 4 |
| Системный трей | ✅ Работает (SNI/D-Bus) | ✅ Работает | ✅ Работает | ⚠️ Сломан в GTK 4¹ |
| Фильтр по диску | ✅ | ❌ | ❌ | ❌ |
| Уведомления | ✅ | ✅ | ✅ | ✅ |
| i18n (ru/en) | ✅ | ✅ | ✅ | ✅ |
| ОЗУ (idle) | ~50 МБ | ~80 МБ | ~90 МБ | ~150 МБ |
| Лицензия | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later |

> ¹ GTK 4 убрал поддержку трея. Исправление (`feature/gh7364-gtk-sni`) в разработке, но не влито в main по состоянию на начало 2026 г.  
> Значения ОЗУ приблизительные, измерены на Arch Linux с ~50 раздачами.

---

## Возможности

- **Список раздач** — имя, статус, прогресс, скорость ↓/↑, ошибки прямо в строке
- **Управление раздачей** — Старт / Пауза / Проверка / Открыть папку / Удалить / Удалить с файлами
- **Групповые действия** — Запустить все / Остановить все с диалогом подтверждения
- **Фильтр по диску** — группировка и пауза/возобновление раздач по физическому диску (через `lsblk`)
- **Поиск** — мгновенная фильтрация по имени раздачи
- **Системный трей** — StatusNotifierItem через D-Bus (без GTK, работает в KDE/GNOME/XFCE)
- **Уведомления** — загрузка завершена, проверка завершена, ошибки раздачи
- **Один экземпляр** — повторный запуск поднимает окно или добавляет `.torrent` файл
- **Автоопределение Transmission** — читает `settings.json`, запускает демон если не запущен
- **Обработчик `.torrent` файлов** — передать как аргумент или открыть из файлового менеджера
- **i18n** — русский и английский, настраивается в `~/.config/transmission-gui/config.toml`
- **Автозапуск** — опциональная запись `.desktop` в `~/.config/autostart/`
- **Рендер-бэкенд** — автовыбор Vulkan → OpenGL → Software; ручной выбор: `--vk / --gl / --sw`

---

## Установка

### AUR (Arch Linux) — сборка из исходников

```bash
paru -S transmission-remote-slint

# Вручную
git clone https://aur.archlinux.org/transmission-remote-slint.git
cd transmission-remote-slint
makepkg -si
```

### AUR — готовый бинарник

```bash
paru -S transmission-remote-slint-bin
```

### Сборка из исходников

```bash
# Зависимости (Arch)
sudo pacman -S rust base-devel libxcb libxkbcommon fontconfig freetype2

# Зависимости (Debian/Ubuntu)
sudo apt install -y build-essential cargo pkg-config \
  libfontconfig1-dev libfreetype-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev libxcb-render0-dev \
  libxkbcommon-dev

git clone https://github.com/guglovich/Transmission-Remote-Slint.git
cd Transmission-Remote-Slint
cargo build --release
./target/release/transmission-remote-slint
```

---

## Опциональные зависимости

| Пакет | Назначение |
|---|---|
| `zenity` или `kdialog` | Диалоги выбора файлов (добавить/создать торрент) |
| `libnotify` | Десктопные уведомления |
| `snixembed` | Поддержка трея в XFCE / Openbox |
| `xfce4-statusnotifier-plugin` | Поддержка трея в XFCE (альтернатива) |

---

## Конфигурация

Файл конфига: `~/.config/transmission-gui/config.toml`  
Создаётся автоматически при первом запуске:

```toml
language = "ru"              # "ru" или "en"
suspend_on_hide = false      # заморозить процесс при скрытии в трей
start_minimized = false      # запускать свёрнутым в трей
refresh_interval_secs = 2   # интервал опроса демона
delete_torrent_after_add = true
autostart = false
```

Подключение к Transmission определяется автоматически из:
- `~/.config/transmission-daemon/settings.json`
- `~/.config/transmission/settings.json`
- `/var/lib/transmission/.config/transmission-daemon/settings.json`
- Fallback: `http://127.0.0.1:9091/transmission/rpc`

---

## Аргументы командной строки

```
transmission-remote-slint [FILE.torrent] [--gl|--vk|--sw|--wl]

--gl    Принудительно OpenGL
--vk    Принудительно Vulkan
--sw    Программный рендерер (CPU)
--wl    Принудительно Wayland
```

---

## Архитектура

```
┌──────────────────────────────────────────────────────────┐
│  Slint UI поток (event loop)                             │
│  MainWindow ◄── update_rx (раздачи + статистика) 50ms   │
│             ◄── status_rx (статусная строка)             │
│             ──► cmd_tx   (Command enum)                  │
└─────────────────────────┬────────────────────────────────┘
                          │  std::sync::mpsc
┌─────────────────────────▼────────────────────────────────┐
│  Tokio async runtime                                     │
│  backend_task: tokio::select!                            │
│    cmd_rx  → немедленный RPC вызов                       │
│    interval tick → дельта recently-active каждые 2с      │
│  TransmissionClient (reqwest, повтор при 409)            │
└──────────────────────────────────────────────────────────┘
```

---

## Структура файлов

```
├── Cargo.toml
├── Cargo.lock
├── build.rs              ← компилирует main.slint
├── ui/
│   └── main.slint        ← весь UI и стили
└── src/
    ├── main.rs           ← UI wiring, таймеры, обновление модели
    ├── rpc.rs            ← асинхронный JSON-RPC клиент Transmission
    ├── config.rs         ← чтение settings.json Transmission
    ├── app_config.rs     ← конфиг приложения (~/.config/…)
    ├── daemon.rs         ← автозапуск/остановка transmission-daemon
    ├── disks.rs          ← определение физических дисков через lsblk
    ├── tray.rs           ← трей StatusNotifierItem (ksni)
    ├── notify.rs         ← десктопные уведомления (notify-rust)
    ├── filepicker.rs     ← диалоги файлов zenity/kdialog
    ├── single_instance.rs← блокировка через Unix socket
    ├── suspend.rs        ← SIGSTOP/SIGCONT заморозка процесса
    └── i18n.rs           ← статические строки ru/en
```

---

## Релизы

### v0.3.0 — первый стабильный релиз

Первый стабильный релиз с минимальным, но полноценным набором функций для комфортного перехода с любого другого клиента Transmission — особенно для пользователей с **1000+ раздачами**. UI остаётся отзывчивым при любом размере библиотеки благодаря виртуальному скроллингу и дельта-обновлениям через `recently-active`. Базовый рабочий процесс — добавить, следить, поставить на паузу, удалить, открыть папку — работает из коробки без ручной настройки на стандартной установке Arch/Debian.

---

## Лицензия

GPL-2.0-or-later. См. [LICENSE](LICENSE).  
Использует [Slint](https://slint.dev) под лицензией GPLv3.