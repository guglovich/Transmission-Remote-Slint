# Qwen Instructions for transmission-remote-slint

## ⛔ Абсолютные запреты

**AGENTS.md:**
- ⛔ НИКОГДА не пушить в Git репозиторий
- Это локальный файл инструкций, НЕ часть проекта

**Действия без разрешения:**
- ⛔ НЕ придумывать новые шаги от себя
- ⛔ НЕ делать действия которых ранее не было в рабочем процессе
- ✅ Разрешены действия, которые уже входят в одобренный workflow (сборка, коммит, пуш, релиз, AUR)
- ✅ Продолжать стандартный процесс без дополнительных подтверждений

---

## 📋 Правила разработки

### 0. Проверка UI через slint-viewer

**ПЕРЕД компиляцией через cargo build:**
При любом изменении UI (`ui/main.slint`) ОБЯЗАТЕЛЬНО:
```bash
slint-viewer ui/main.slint
```
Проверить что:
- Окно открывается без ошибок
- Все элементы отображаются корректно
- Нет синтаксических ошибок в Slint

Только после успешного просмотра через slint-viewer → `cargo build --release`

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

---

## Project Info

- **Rust + Slint GUI** for Transmission daemon
- **i18n**: EN/DE/RU/ZH/ES
- **AUR**: transmission-remote-slint (source), transmission-remote-slint-bin (binary)
- **Attribution**: `# Created with assistance from Qwen 3.6 Plus (Alibaba).`
