# Transmission Remote — Slint

Лёгкий нативный графический клиент для **Transmission daemon**, написанный на **Rust + Slint**.  
Без GTK, без Qt — рендеринг через Skia/OpenGL или Vulkan.

> **Разработан с помощью Qwen 3.5 Plus (Alibaba).**

---

## Новое в v0.5

### 🌐 Мультиязычность
- **5 языков:** English, Deutsch, Русский, 中文, Español
- Выбор языка через диалог настроек (кнопка ⚙ в toolbar)
- Сохранение выбора языка между запусками
- Перевод меню системного трея

### 🖥️ Новый интерфейс
- **Современная панель инструментов:**
  - Кнопки ➕ Открыть, 🧲 Magnet, 📀 Создать, 🔄 Rehash слева
  - Поле поиска по центру
  - Кнопка настроек (⚙) справа с текущим языком
- **Левая панель:**
  - Фильтры статусов: Все, Загружаются, Раздаются, Завершены, Остановлены, Активные, Неактивные, Проверяются, Ошибка
  - Счётчик торрентов для каждого статуса
  - Клик для фильтрации по статусу
- **Диалог настроек:**
  - Выбор языка с флагами
  - Уведомление о необходимости перезапуска

### 🔧 Системный трей
- **Запустить все / Остановить все** для массового управления
- Визуальная индикация статуса (кнопки показывают текущее состояние)

### 🛠️ Другие улучшения
- Magnet ссылки через диалог ввода + xdg-open
- Выбор .torrent файлов + xdg-open интеграция
- Исправлен баг с двойным Cancel в CreateTorrentDialog

---

## Возможности

- **Список раздач** — имя, статус, прогресс, ↓/↑ скорость, текст ошибки прямо в строке
- **Действия над раздачей** — Старт / Пауза / Перепроверить / Открыть папку / Удалить / Удалить с файлами
- **Массовые операции** — Запустить все / Остановить все с диалогом подтверждения
- **Фильтры статусов** — фильтрация раздач по статусу (Все, Загружаются, Раздаются, и т.д.)
- **Мгновенный поиск** — фильтрация по имени без ожидания RPC
- **Системный трей** — StatusNotifierItem через D-Bus (нативный zbus 4, без ksni/GTK)
- **Уведомления рабочего стола** — завершение загрузки, конец проверки, ошибки раздач
- **Одиночный экземпляр** — повторный запуск поднимает окно или добавляет `.torrent` файл
- **Авто-определение Transmission** — читает статус демона, подключается автоматически
- **Открытие `.torrent` файлов** — из файлового менеджера или как аргумент командной строки
- **Мультиязычность** — 5 языков, настройка через диалог настроек
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

### Ручная установка

```bash
git clone https://github.com/guglovich/Transmission-Remote-Slint.git
cd Transmission-Remote-Slint
cargo build --release
sudo cp target/release/transmission-remote-slint /usr/local/bin/
```

---

## Конфигурация

Файл конфигурации: `~/.config/transmission-remote-slint/config.toml`

```toml
# URL Transmission демона
url = "http://localhost:9091"

# Аутентификация (опционально)
# username = "transmission"
# password = "transmission"

# Язык (en, de, ru, zh, es)
language = "ru"

# Интервал обновления в секундах
refresh_interval_secs = 2

# Запуск свёрнутым в трей
start_minimized = false

# Приостанавливать при скрытии (экономия ресурсов)
suspend_on_hide = false

# Удалять .torrent файл после добавления
delete_torrent_after_add = true

# Автозапуск при входе в систему
autostart = false
```

---

## Использование

### Горячие клавиши

| Клавиша | Действие |
|-----|--------|
| `Ctrl+O` | Открыть .torrent файл |
| `Ctrl+M` | Добавить magnet ссылку |
| `Ctrl+S` | Открыть настройки |
| `Escape` | Закрыть диалог |

### Системный трей

- **Левый клик** — Показать/Скрыть окно
- **Правый клик** — Контекстное меню:
  - Показать / Скрыть
  - Запустить все
  - Остановить все
  - Выход

---

## Требования

### Для запуска

- `transmission-daemon` — запущенный демон Transmission
- `libxcb` — библиотека протокола X11
- `libxkbcommon` — библиотека ключевых карт XKB
- `fontconfig` — конфигурация шрифтов
- `freetype2` — рендеринг шрифтов
- `dbus` — шина для уведомлений и трея

### Опционально

- `zenity` — диалоги выбора файлов (GNOME/X11)
- `kdialog` — диалоги выбора файлов (KDE)
- `yad` — диалоги выбора файлов (альтернатива)
- `libnotify` — уведомления рабочего стола
- `snixembed` / `xfce4-statusnotifier-plugin` — поддержка системного трея

### Для сборки

- `rust` — компилятор Rust
- `cargo` — менеджер пакетов Rust
- `pkg-config` — помощник конфигурации сборки

---

## Сборка из исходников

```bash
# Клонировать репозиторий
git clone https://github.com/guglovich/Transmission-Remote-Slint.git
cd Transmission-Remote-Slint

# Собрать релиз
cargo build --release

# Запустить
./target/release/transmission-remote-slint
```

### Сборка с определённым рендерером

```bash
# OpenGL (по умолчанию)
cargo build --release --features backend-winit,renderer-winit-skia-opengl

# Vulkan
cargo build --release --features backend-winit,renderer-winit-skia-vulkan

# Программный (резервный)
cargo build --release --features backend-winit,renderer-winit-software
```

---

## Решение проблем

### Трей не отображается

Установите поддержку системного трея:
- **XFCE:** `xfce4-statusnotifier-plugin`
- **Openbox:** `snixembed`
- **GNOME:** `gnome-shell-extension-appindicator`

### Уведомления не работают

Установите libnotify:
```bash
sudo pacman -S libnotify
```

### Не открывается выбор файлов

Установите утилиту диалогов:
```bash
# GNOME/X11
sudo pacman -S zenity

# KDE
sudo pacman -S kdialog

# Альтернатива
sudo pacman -S yad
```

### Не удаётся подключиться к демону

1. Убедитесь что демон Transmission запущен:
   ```bash
   systemctl --user start transmission
   ```

2. Проверьте URL демона в конфиге:
   ```toml
   url = "http://localhost:9091"
   ```

3. Проверьте учётные данные если требуется аутентификация.

---

## Сравнение

| Функция | **transmission-remote-slint v0.5** | transmission-remote-gtk | transmission-qt | Transmission GTK 4.x |
|---|---|---|---|---|
| Тип | Только удалённый | Только удалённый | Standalone + Remote | Standalone |
| Тулкит | Slint (Rust) | GTK 3 | Qt 5/6 | GTK 4 |
| Системный трей | ✅ Работает (SNI/D-Bus) | ✅ Работает | ✅ Работает | ⚠️ Сломан в GTK 4¹ |
| Уведомления | ✅ | ✅ | ✅ | ✅ |
| ОЗУ (простой) | ~50 МБ | ~80 МБ | ~90 МБ | ~150 МБ |
| Языки | 5 | 1 | Несколько | Несколько |
| Лицензия | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later |

> ¹ GTK 4 убрал поддержку трея. Исправление разрабатывается, но не влито по состоянию на начало 2026 года.  
> Цифры ОЗУ приблизительные, измерены на Arch Linux с ~50 раздачами.

---

## Скриншоты

### v0.5 Главное окно

![Главное окно](screenshots/main-v0.5.png)

### Диалог настроек

![Настройки](screenshots/settings-v0.5.png)

### Меню трея

![Меню трея](screenshots/tray-v0.5.png)

---

## Планы

### v0.6 (планируется)
- [ ] Фильтр по дискам (вернуть с улучшенным UI)
- [ ] Диалог свойств раздачи
- [ ] Контроль ограничения скорости
- [ ] Управление очередью
- [ ] Переключатель тёмной/светлой темы

### Будущее
- [ ] Мастер создания торрентов
- [ ] Список пиров
- [ ] Контроль приоритета файлов
- [ ] Поддержка RSS лент
- [ ] Больше переводов

---

## Лицензия

GPL-2.0-or-later — см. [LICENSE](LICENSE) для деталей.

---

## Благодарности

- **Transmission** — BitTorrent демон
- **Slint** — UI тулкит
- **Qwen 3.5 Plus** — AI помощник в разработке
- **AUR мейнтейнеры** — поддержка пакетов
