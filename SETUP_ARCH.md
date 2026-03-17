# Настройка на Arch Linux

## Быстрый старт

```bash
# Установка (если ещё не установлено)
sudo pacman -S transmission-cli

# Запуск демона вручную (для теста)
transmission-daemon

# Проверка что RPC работает
transmission-remote localhost:9091 --session-info
```

## Если видите "403 Forbidden" или "Got HTML"

Демон запущен, но блокирует localhost. Отредактируйте `~/.config/transmission-daemon/settings.json`
(**остановите демон перед редактированием** — он перезаписывает файл при выходе):

```bash
transmission-remote localhost:9091 --exit   # или: pkill transmission-daemon
```

Убедитесь что в settings.json есть:
```json
"rpc-whitelist-enabled": false,
"rpc-authentication-required": false,
"rpc-bind-address": "0.0.0.0",
"rpc-port": 9091,
"rpc-url": "/transmission/"
```

Затем запустите снова: `transmission-daemon`

## Systemd сервис (опционально)

```bash
# Пользовательский сервис (рекомендуется)
systemctl --user enable --now transmission

# Системный сервис (как пользователь transmission)
sudo systemctl enable --now transmission
```

Если используете системный сервис с пользователем `transmission`,
конфиг находится в `/var/lib/transmission/.config/transmission-daemon/settings.json`.

## Сборка GUI

```bash
sudo pacman -S base-devel  # если нет
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

cd transmission-gui
cargo run --release
```
