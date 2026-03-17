// src/i18n.rs — статическая мультиязычность

use std::sync::OnceLock;

static LANG: OnceLock<Lang> = OnceLock::new();

#[derive(Debug, Clone, PartialEq)]
pub enum Lang { Ru, En }

pub fn init(lang: &str) {
    let l = match lang.to_lowercase().as_str() {
        "en" | "english" => Lang::En,
        _                => Lang::Ru,
    };
    let _ = LANG.set(l);
}

fn lang() -> &'static Lang {
    LANG.get().unwrap_or(&Lang::Ru)
}

macro_rules! t {
    ($ru:expr, $en:expr) => {
        if lang() == &Lang::Ru { $ru } else { $en }
    };
}

// ── Статусы торрентов ─────────────────────────────────────────────────────────

pub fn status_stopped()     -> &'static str { t!("Приостановлено",       "Stopped")      }
pub fn status_check_wait()  -> &'static str { t!("Очередь проверки",     "Check Wait")   }
pub fn status_checking()    -> &'static str { t!("Проверка",             "Checking")     }
pub fn status_dl_queue()    -> &'static str { t!("Очередь загрузки",     "DL Queue")     }
pub fn status_downloading() -> &'static str { t!("Загрузка",             "Downloading")  }
pub fn status_seed_queue()  -> &'static str { t!("Очередь раздачи",      "Seed Queue")   }
pub fn status_seeding()     -> &'static str { t!("Раздача",              "Seeding")      }
pub fn status_unknown()     -> &'static str { t!("Неизвестно",           "Unknown")      }

pub fn err_tracker_warn()   -> &'static str { t!("Предупреждение трекера","Tracker warn") }
pub fn err_tracker_err()    -> &'static str { t!("Ошибка трекера",        "Tracker err")  }
pub fn err_missing()        -> &'static str { t!("Нет данных",            "No data")      }
pub fn err_generic()        -> &'static str { t!("Ошибка",                "Error")        }
