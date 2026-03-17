// src/single_instance.rs
// Single-instance через Unix domain socket.
// Первый запуск — слушает сокет, принимает файлы от последующих запусков.
// Второй запуск — отправляет путь к файлу первому и выходит.

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/tmp"));
    PathBuf::from(runtime_dir).join("transmission-gui.sock")
}

pub enum InstanceRole {
    /// Мы первый экземпляр — нужно слушать сокет
    Primary(UnixListener),
    /// Уже запущен другой экземпляр — файл передан, можно выходить
    Secondary,
}

/// Определяем роль этого процесса.
/// `torrent_path` — опциональный .torrent файл из аргументов.
pub fn acquire(torrent_path: Option<&str>) -> InstanceRole {
    let path = socket_path();

    // Пробуем подключиться к уже работающему экземпляру
    if let Ok(mut stream) = UnixStream::connect(&path) {
        // Есть живой экземпляр — передаём ему файл (или пустую строку для фокуса)
        let msg = torrent_path.unwrap_or("").as_bytes();
        let _ = stream.write_all(msg);
        eprintln!("[single_instance] Secondary — sent to primary, exiting");
        return InstanceRole::Secondary;
    }

    // Сокет мёртвый или не существует — удаляем и создаём новый
    let _ = std::fs::remove_file(&path);
    match UnixListener::bind(&path) {
        Ok(listener) => {
            eprintln!("[single_instance] Primary — listening on {}", path.display());
            InstanceRole::Primary(listener)
        }
        Err(e) => {
            eprintln!("[single_instance] Could not bind socket: {e}, proceeding as primary anyway");
            // Fallback: создаём фиктивный listener через /dev/null пути
            // Просто продолжаем без single-instance защиты
            let tmp = PathBuf::from("/tmp/transmission-gui-fallback.sock");
            let _ = std::fs::remove_file(&tmp);
            UnixListener::bind(&tmp)
                .map(InstanceRole::Primary)
                .unwrap_or(InstanceRole::Secondary)
        }
    }
}

/// Запускаем фоновый поток который принимает сообщения от вторичных экземпляров.
/// `on_torrent` вызывается когда приходит путь к .torrent файлу.
pub fn start_listener(listener: UnixListener, on_torrent: impl Fn(String) + Send + 'static) {
    std::thread::spawn(move || {
        listener.set_nonblocking(false).ok();
        for stream in listener.incoming() {
            match stream {
                Ok(mut s) => {
                    let mut buf = Vec::new();
                    if s.read_to_end(&mut buf).is_ok() {
                        let msg = String::from_utf8_lossy(&buf).trim().to_string();
                        eprintln!("[single_instance] Received: {msg:?}");
                        if !msg.is_empty() && msg.ends_with(".torrent") {
                            on_torrent(msg);
                        } else {
                            // Пустое сообщение — просто поднять окно
                            on_torrent(String::new());
                        }
                    }
                }
                Err(e) => eprintln!("[single_instance] Accept error: {e}"),
            }
        }
    });
}
