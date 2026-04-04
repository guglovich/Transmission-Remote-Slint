# Transmission Remote Slint — AGENTS.md

## 🎯 Цель проекта

**Создать лёгкий, быстрый нативный GUI для Transmission daemon без GTK/Qt зависимостей.**

### От чего ушли:
- ❌ GTK/Qt зависимости и накладные расходы
- ❌ Блокировки UI потока во время обновлений
- ❌ Зависания интерфейса при 100+ торрентах
- ❌ Сложные системы сборки

### К чему идём:
- ✅ **Минимальные зависимости** — только Rust + Slint
- ✅ **Lock-free архитектура** — `std::sync::mpsc` каналы между UI и async бэкендом
- ✅ **Производительность** — 500ms poll без блокировок, ~50 MB RAM при 50+ торрентах
- ✅ **Мультиязычность** — 5 языков (EN/DE/RU/ZH/ES) с простым словарём
- ✅ **AUR пакеты** — source + binary для Arch Linux

---

## 📋 Правила разработки (v0.6+)

### 0. Абсолютные запреты

**AGENTS.md:**
- ⛔ НИКОГДА не пушить в Git репозиторий
- Это локальный файл инструкций, НЕ часть проекта

**Действия без разрешения:**
- ⛔ НЕ придумывать новые шаги от себя
- ⛔ НЕ делать действия которых ранее не было в рабочем процессе
- ✅ Разрешены действия, которые уже входят в одобренный workflow (сборка, коммит, пуш, релиз, AUR)
- ✅ Продолжать стандартный процесс без дополнительных подтверждений

### 1. Критически важные правила релизов

**ЗАГРУЗКА БИНАРНИКОВ В КАЖДЫЙ РЕЛИЗ:**
После создания `gh release create vX.Y.Z` ОБЯЗАТЕЛЬНО:
```bash
# 1. Собрать бинарник ИЗ ЭТОГО ТЕГА (НЕ из текущей ветки!)
git checkout vX.Y.Z
cargo build --release

# 2. Загрузить в СООТВЕТСТВУЮЩИЙ релиз
gh release upload vX.Y.Z target/release/transmission-remote-slint --clobber

# 3. Вернуться на main
git checkout main
```

⚠️ КАЖДЫЙ релиз должен содержать СВОЙ бинарник, собранный из СВОЕГО кода!
⚠️ НИКОГДА не загружать один бинарник в несколько релизов!
⚠️ ВСЕГДА собирать из тега, НЕ из текущей ветки!

### 1. Обязательные extensions

При любой разработке **всегда использовать**:

```markdown
## Required Extensions
- `context7` — документация библиотек (Slint, tokio, reqwest, etc.)
- `rust-agentic-skills` — Rust best practices, safety checks
- `superpowers` — общие навыки разработки
- `ui-ux-pro-max` — UI/UX дизайн, accessibility
- `basic-memory` — память между сессиями
- `slint-lsp-helper` — Slint UI линтинг, проверка синтаксиса
```

### 2. Архитектурные принципы

#### 2.1 Lock-free коммуникация
```rust
// ✅ ПРАВИЛЬНО: std::sync::mpsc каналы
let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
let (update_tx, update_rx) = sync_channel::<Update>(8);

// ❌ НЕПРАВИЛЬНО: Arc<Mutex<>> для частых обновлений UI
```

#### 2.2 Async backend + sync UI
```rust
// Tokio runtime для RPC
let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

// Slint UI thread (event loop)
_tmr_fast.start(slint::TimerMode::Repeated, Duration::from_millis(50), { ... });
_tmr_data.start(slint::TimerMode::Repeated, Duration::from_millis(1000), { ... });
```

#### 2.3 i18n через простой словарь
```rust
// ✅ ПРАВИЛЬНО: static словарь в i18n.rs
pub fn toolbar_open() -> &'static str { 
    tr!("Open", "Öffnen", "Открыть", "打开", "Abrir") 
}

// ❌ НЕПРАВИЛЬНО: external crates (gettext, fluent)
```

### 3. UI/UX правила

#### 3.1 Slint структура
```slint
// ✅ ПРАВИЛЬНО: минимальная вложенность для производительности
component MainWindow inherits Window {
    in property <[TorrentItem]> torrents;
    in property <[DiskCacheEntry]> disk-cache;
    
    // Callbacks
    callback filter-clicked(string);
    callback language-changed(string);
    
    // UI элементы
    Rectangle {
        HorizontalLayout {
            // Кнопки с alignment: center
        }
    }
}
```

#### 3.2 Производительность ListView
```slint
// ✅ ПРАВИЛЬНО: фиксированная высота, без вложенных Layout
ListView {
    vertical-stretch: 1;
    for item[idx] in torrents : TorrentRow {
        item: item;
        is-even: mod(idx, 2) == 0;
        // callbacks...
    }
}

component TorrentRow inherits Rectangle {
    height: 50px;  // Фиксированная высота
    // Прямые Text элементы, не HorizontalLayout внутри
}
```

#### 3.3 Доступность
- Все кнопки имеют текстовые метки
- Цвета соответствуют WCAG contrast (4.5:1)
- Keyboard navigation через Slint по умолчанию

### 4. Код стайл

#### 4.1 Rust
```rust
// Следовать rustfmt
// Использовать anyhow::Result для ошибок
// eprintln! для логирования (не println!)

// ✅ ПРАВИЛЬНО:
fn load_config() -> anyhow::Result<AppConfig> {
    let path = config_path();
    let text = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&text)?)
}

// ❌ НЕПРАВИЛЬНО: unwrap() без контекста
```

#### 4.2 Именование
```rust
// Префиксы для типов каналов
let cmd_tx, cmd_rx  // Command sender/receiver
let update_tx, update_rx  // Update sender/receiver

// Суффиксы для Arc/Mutex
let disk_cache: Arc<Mutex<Vec<DiskCacheEntry>>>
let torrent_snapshot: Arc<Mutex<Vec<RawTorrent>>>
```

### 5. AUR пакеты

#### 5.1 Версионирование
```bash
# ✅ ПРАВИЛЬНО: pkgver соответствует GitHub тегу
pkgver=0.5  # GitHub tag: v0.5

# ❌ НЕПРАВИЛЬНО: pkgver=0.5.0 если тег v0.5
```

#### 5.2 Два пакета
```bash
transmission-remote-slint      # build from source
transmission-remote-slint-bin  # prebuilt binary from GitHub Releases
```

#### 5.3 PKGBUILD правила
- Всегда указывать `# Created with assistance from Qwen 3.6 Plus (Alibaba).`
- sha256sums обновлять при каждом релизе
- .SRCINFO регенерировать через `makepkg --printsrcinfo`

### 6. GitHub релизы

#### 6.1 Release checklist
```markdown
## v0.X.0 — YYYY-MM-DD

### 🌐 Internationalization
- [ ] 5 languages (EN/DE/RU/ZH/ES)
- [ ] Tray menu translated

### 🖥️ UI Changes
- [ ] Toolbar updates
- [ ] New dialogs

### 🔧 System Tray
- [ ] Resume All / Pause All

### 🛠️ Improvements
- [ ] Bug fixes
- [ ] Performance

### 📦 Installation
AUR:
- paru -S transmission-remote-slint
- paru -S transmission-remote-slint-bin

---
**Built with Qwen 3.6 Plus**
```

#### 6.2 Binary upload
```bash
# Release binary для transmission-remote-slint-bin
cargo build --release
gh release upload v0.5 target/release/transmission-remote-slint
```

### 7. Документация

#### 7.1 README структура
```markdown
# Title
Languages: EN | RU

## Comparison (таблица)
## Features (список)
## Installation (AUR + build)
## Configuration (config.toml)
## Command-line options
## Architecture (диаграмма)
## File structure
## License (с компонентами)
## Release Notes (ссылка на GitHub)
```

#### 7.2 Запрещено
- ❌ Упоминания RAM usage (были проблемы)
- ❌ Performance benchmarks без тестов
- ❌ Screenshots без обсуждения

### 8. CI/CD (планируется)

```yaml
# .github/workflows/release.yml (будущее)
- Build binary for GitHub Releases
- Update AUR packages automatically
- Run clippy, rustfmt checks
```

---

## 📚 Полезные ссылки

- **Slint Docs:** https://slint.dev/docs
- **Slint LSP:** `slint-lsp` для линтинга
- **AUR Submission:** https://aur.archlinux.org/packages
- **GitHub Releases:** https://github.com/guglovich/Transmission-Remote-Slint/releases

---

## 🤖 AI Assistant Notes

**Developed with Qwen 3.6 Plus (Alibaba).**

При продолжении разработки:
1. Всегда читать этот файл перед началом работы
2. Использовать указанные extensions
3. Следовать архитектурным принципам
4. Проверять slint-lsp перед коммитом
5. Обновлять AUR при релизе

---

**Last updated:** 2026-03-31 (v0.5 release)
