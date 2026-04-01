# Transmission Remote — Slint

Лёгкий нативный графический клиент для **Transmission daemon**, написанный на **Rust + Slint**.  
Без GTK, без Qt — рендеринг через Skia/OpenGL или Vulkan.

> **Разработан с помощью Qwen 3.5 Plus (Alibaba).**

**Языки:** Русский | [English](README.md)

---

## Сравнение производительности UI

GTK и Qt фронтенды страдают от одной и той же хорошо известной проблемы при большой библиотеке торрентов. Оба рендерят список на **главном UI-потоке** и перестраивают всю модель при каждом опросе. GTK 4 ведёт себя особенно агрессивно: вызывает `gtk_list_store_clear()` и повторно вставляет все строки каждые несколько секунд, из-за чего главный цикл GTK полностью зависает.

Реальные репорты это подтверждают:

- **GTK 4.1 с ~4 700 торрентами** — один клик занимает до минуты; артефакты окна появляются поверх других приложений. ([#8359](https://github.com/transmission/transmission/issues/8359))
- **Qt и GTK с 3 200+ торрентами** — поиск, открытие или изменение торрента могут занимать часы. ([#4193](https://github.com/transmission/transmission/issues/4193))

Qt-клиент на практике ведёт себя несколько лучше, потому что `QAbstractItemModel` с сигналами `dataChanged` позволяет точечно обновлять ячейки без полного сброса. Но корень проблемы остаётся: и опрос, и обновление модели происходят на главном потоке, и при тысячах активных торрентов с частыми обновлениями цикл событий переполняется. Issue #4193, затрагивающий и GTK, и Qt, был закрыт как регрессия в ядре — не исправление фронтенда.

**Этот проект устроен принципиально иначе:**

```
┌──────────────────────────────────────────────────────────┐
│  Slint UI thread (цикл событий)                          │
│  MainWindow ◄── update_rx (торренты + статистика) 50ms  │
│             ◄── status_rx (текст статус-бара)            │
│             ──► cmd_tx   (Command enum)                  │
└─────────────────────────┬────────────────────────────────┘
                          │  std::sync::mpsc
┌─────────────────────────▼────────────────────────────────┐
│  Tokio async runtime                                     │
│  backend_task: tokio::select!                            │
│    cmd_rx  → немедленный RPC-вызов                       │
│    interval tick → recently-active дельта каждые 2с      │
│  TransmissionClient (reqwest, retry по 409)              │
└──────────────────────────────────────────────────────────┘
```

- **Tokio async runtime** обрабатывает весь сетевой I/O в отдельном потоке — UI никогда не блокируется на RPC-вызовах
- **`recently-active` дельта-обновления** — запрашиваются и передаются в UI только торренты, изменившиеся за последний интервал; полный список никогда не перерисовывается без явного запроса
- **Виртуальный скроллинг Slint** — рендерятся только видимые строки, вне зависимости от размера библиотеки
- UI-поток получает только небольшой diff через канал `mpsc` и применяет его; он никогда не выходит в сеть

Результат: UI остаётся отзывчивым при 1 000+ и 4 000+ торрентах, потому что главный поток попросту никогда не делает ту работу, которая убивает GTK и Qt при масштабировании.

---

## Сравнение

| Функция | **transmission-remote-slint** | transmission-remote-gtk | transmission-qt | Transmission GTK 4.x |
|---|---|---|---|---|
| Тип | Только remote | Только remote | Standalone + Remote | Standalone |
| Тулкит | Slint (Rust) | GTK 3 | Qt 5/6 | GTK 4 |
| UI-поток блокируется на опросе? | ✅ Никогда | ❌ Всегда | ⚠️ Частично | ❌ Всегда |
| Стратегия обновлений | `recently-active` дельта | Полная перестройка | Частично через сигналы | Полная перестройка |
| Виртуальный скроллинг | ✅ | ❌ | ❌ | ❌ |
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

Файл конфигурации: `~/.config/transmission-remote-slint/config.toml`  
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
│  MainWindow ◄── update_rx (torrents + stats)  50ms pump │
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
    ├── wm_icon.rs         ← _NET_WM_ICON иконка в панели задач (X11)
    ├── suspend.rs         ← SIGSTOP/SIGCONT приостановка процесса
    └── i18n.rs            ← мультиязычные статические строки (5 языков)
```

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
