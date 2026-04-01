# Transmission Remote — Slint

Лёгкий нативный графический клиент для **Transmission daemon**, написанный на **Rust + Slint**.  
Без GTK, без Qt — рендеринг через Skia/OpenGL или Vulkan.

> **Разработан с помощью Qwen 3.5 Plus (Alibaba).**

**Языки:** Русский | [English](README.md)

---

## Сравнение

| Функция | **transmission-remote-slint** | transmission-remote-gtk | transmission-qt | Transmission GTK 4.x |
|---|---|---|---|---|
| Тип | Только удалённый | Только удалённый | Standalone + Remote | Standalone |
| Тулкит | Slint (Rust) | GTK 3 | Qt 5/6 | GTK 4 |
| Системный трей | ✅ Работает (SNI/D-Bus) | ✅ Работает | ✅ Работает | ⚠️ Сломан в GTK 4¹ |
| Лицензия | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later |

> ¹ GTK 4 убрал поддержку трея. Исправление разрабатывается, но не влито по состоянию на начало 2026 года.

---

## Возможности

- **Список раздач** — имя, статус, прогресс, ↓/↑ скорость, текст ошибки прямо в строке
- **Действия над раздачей** — Старт / Пауза / Перепроверить / Открыть папку / Удалить / Удалить с файлами
- **Массовые операции** — Запустить все / Остановить все с диалогом подтверждения
- **Фильтры статусов** — фильтрация раздач по статусу (Все, Загружаются, Раздаются, Завершены, Остановлены, Активные, Неактивные, Проверяются, Ошибка)
- **Мгновенный поиск** — фильтрация по имени без ожидания RPC
- **Системный трей** — StatusNotifierItem через D-Bus (нативный zbus 4, без ksni/GTK)
  - Кнопки Запустить все / Остановить все
  - Переведённое меню
- **Уведомления рабочего стола** — завершение загрузки, конец проверки, ошибки раздач
- **Одиночный экземпляр** — повторный запуск поднимает окно или добавляет `.torrent` файл
- **Авто-определение Transmission** — читает статус демона, подключается автоматически
- **Открытие `.torrent` файлов** — из файлового менеджера или как аргумент командной строки
- **Magnet ссылки** — диалог ввода + xdg-open
- **Мультиязычность** — 5 языков (EN/DE/RU/ZH/ES), настройка через диалог настроек
- **Иконка приложения** — встроена в бинарник, устанавливается в hicolor тему через PKGBUILD
- **Автозапуск** — опциональный `.desktop` файл в `~/.config/autostart/`
- **Бэкенд рендеринга** — автовыбор Vulkan → OpenGL → Программный

---

## Установка

### AUR (Arch Linux) — сборка из исходников

```bash
paru -S transmission-remote-slint
# или вручную:
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
# Arch
sudo pacman -S rust base-devel libxcb libxkbcommon fontconfig freetype2

# Debian/Ubuntu
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

## Опциональные зависимости для запуска

| Пакет | Назначение |
|---|---|
| `zenity` или `kdialog` | Диалоги выбора файлов |
| `libnotify` | Уведомления рабочего стола |
| `snixembed` | Поддержка трея в XFCE / Openbox |
| `xfce4-statusnotifier-plugin` | Поддержка трея в XFCE (альтернатива) |
| `xdotool` | Иконка в панели задач через `_NET_WM_ICON` |

---

## Конфигурация

Файл конфигурации: `~/.config/transmission-gui/config.toml`  
Создаётся автоматически при первом запуске:

```toml
language = "ru"                 # "en", "de", "ru", "zh", "es"
suspend_on_hide = false         # заморозка процесса при сворачивании в трей
start_minimized = false         # запуск скрытым в трее
refresh_interval_secs = 2       # интервал опроса
delete_torrent_after_add = true # удалять .torrent файл после добавления (как в Transmission GTK)
autostart = false
```

Подключение к Transmission определяется автоматически из статуса демона.

---

## Опции командной строки

```
transmission-remote-slint [ФАЙЛ.torrent] [--gl|--vk|--sw|--wl]

--gl    Принудительно OpenGL рендерер
--vk    Принудительно Vulkan рендерер
--sw    Принудительно программный рендерер (CPU)
--wl    Принудительно Wayland бэкенд
```

---

## Архитектура

```
┌──────────────────────────────────────────────────────────┐
│  Slint UI thread (event loop)                            │
│  MainWindow ◄── update_rx (torrents + stats)  500ms pump │
│             ◄── status_rx (status bar text)              │
│             ──► cmd_tx   (Command enum)                  │
└─────────────────────────┬────────────────────────────────┘
                          │  std::sync::mpsc
┌─────────────────────────▼────────────────────────────────┐
│  Tokio async runtime                                     │
│  backend_task: tokio::select!                            │
│    cmd_rx  → immediate RPC call                          │
│    interval tick → recently-active delta every 2s        │
│  TransmissionClient (reqwest, 409 session retry)         │
└──────────────────────────────────────────────────────────┘
```

---

## Структура файлов

```
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── PKGBUILD
├── .SRCINFO
├── ui/
│   ├── main.slint
│   └── app-icon.png
└── src/
    ├── main.rs            ← привязка UI, таймеры, обновление моделей
    ├── rpc.rs             ← асинхронный Transmission JSON-RPC клиент
    ├── config.rs          ← читает Transmission settings.json
    ├── app_config.rs      ← конфигурация приложения
    ├── daemon.rs          ← авто-запуск/остановка transmission-daemon
    ├── disks.rs           ← определение физических дисков через lsblk
    ├── tray.rs            ← StatusNotifierItem (нативный zbus 4)
    ├── notify.rs          ← уведомления рабочего стола
    ├── filepicker.rs      ← zenity/kdialog диалоги выбора файлов
    ├── single_instance.rs ← Unix socket блокировка одиночного экземпляра
    ├── wm_icon.rs         ← иконка в панели задач _NET_WM_ICON (X11)
    ├── suspend.rs         ← SIGSTOP/SIGCONT приостановка процесса
    └── i18n.rs            ← мультиязычные статические строки (5 языков)
```

---

## English Documentation

See [README.md](README.md)

---

## License

GPL-2.0-or-later. См. [LICENSE](LICENSE).

### Лицензии компонентов:
- **Slint** — GPLv3 (UI тулкит)
- **zbus** — MIT/Apache-2.0 (D-Bus)
- **tokio** — MIT (async runtime)
- **reqwest** — MIT/Apache-2.0 (HTTP клиент)
- **serde** — MIT/Apache-2.0 (сериализация)
- **image** — MIT (обработка иконок)

---

## Примечания к релизам

См. [GitHub Releases](https://github.com/guglovich/Transmission-Remote-Slint/releases) для списка изменений.
