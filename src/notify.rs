// src/notify.rs — desktop уведомления через D-Bus (notify-rust / libnotify)
//
// События:
//   - Раздача завершена (percentDone 1.0 + status Seeding)
//   - Рехеш завершён   (status Check/CheckWait → Stopped/Seeding)
//   - Раздача сломана  (error > 0, впервые)

use notify_rust::{Notification, Urgency};

/// Отправить уведомление в фоновом треде — не блокирует event loop
fn send(summary: &str, body: &str, urgency: Urgency) {
    let summary = summary.to_owned();
    let body    = body.to_owned();
    std::thread::spawn(move || {
        let result = Notification::new()
            .summary(&summary)
            .body(&body)
            .icon("transmission")
            .appname("Transmission Remote")
            .urgency(urgency)
            .timeout(notify_rust::Timeout::Milliseconds(10000))
            .show();
        if let Err(e) = result {
            eprintln!("[notify] Failed to send notification: {e}");
        }
    });
}

pub fn torrent_finished(name: &str) {
    eprintln!("[notify] Finished: {name}");
    send(
        "Download complete",
        name,
        Urgency::Normal,
    );
}

pub fn torrent_error(name: &str, error_code: i64) {
    let reason = match error_code {
        1 => "Tracker warning",
        2 => "Tracker error",
        3 => "Missing files / local error",
        _ => "Unknown error",
    };
    eprintln!("[notify] Error on '{name}': {reason}");
    send(
        &format!("Torrent error — {reason}"),
        name,
        Urgency::Critical,
    );
}

pub fn recheck_finished(name: &str) {
    eprintln!("[notify] Recheck done: {name}");
    send(
        "Recheck complete",
        name,
        Urgency::Low,
    );
}

// ── Трекер предыдущих состояний ──────────────────────────────────────────────

#[derive(Default)]
pub struct NotifyTracker {
    /// id → (percent_done * 1000 as u32, status, error)
    prev: std::collections::HashMap<i64, TorrentState>,
}

#[derive(Clone)]
struct TorrentState {
    percent: u32,   // percentDone * 1000, ceil — 1000 = 100%
    status:  i64,
    error:   i64,
}

impl NotifyTracker {
    /// Вызывать каждый раз при получении нового списка торрентов.
    /// torrents: срез (id, name, percentDone, status, error)
    pub fn update(&mut self, torrents: &[(i64, &str, f64, i64, i64)]) {
        for &(id, name, percent, status, error) in torrents {
            let pct = (percent * 1000.0).round() as u32;

            if let Some(prev) = self.prev.get(&id) {
                // Завершение раздачи: percent стал 1.0 (1000) и стало Seeding (6)
                if prev.percent < 1000 && pct >= 1000 && status == 6 {
                    torrent_finished(name);
                }

                // Рехеш завершён: был Check Wait (1) или Checking (2), стал чем-то другим
                if (prev.status == 1 || prev.status == 2)
                    && status != 1 && status != 2
                {
                    recheck_finished(name);
                }

                // Новая ошибка: error появился впервые (не было раньше)
                if prev.error == 0 && error > 0 {
                    torrent_error(name, error);
                }
            }
            // При первом появлении торрента — только запоминаем, не уведомляем.
            // Иначе при старте приложения будет шквал уведомлений.

            self.prev.insert(id, TorrentState { percent: pct, status, error });
        }

        // Удаляем из трекера торренты которых больше нет
        let ids: std::collections::HashSet<i64> = torrents.iter().map(|t| t.0).collect();
        self.prev.retain(|id, _| ids.contains(id));
    }
}
