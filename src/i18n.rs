// src/i18n.rs — простой словарь переводов для v0.5
// Переводы основаны на Transmission CLI/Desktop

use std::sync::OnceLock;

static LANG: OnceLock<Lang> = OnceLock::new();

#[derive(Debug, Clone, PartialEq)]
pub enum Lang { En, De, Ru, Zh, Es }

pub fn init(lang: &str) {
    set_language(lang);
}

pub fn set_language(lang: &str) {
    let locale = match lang.to_lowercase().as_str() {
        "en" => Lang::En,
        "de" => Lang::De,
        "ru" => Lang::Ru,
        "zh" => Lang::Zh,
        "es" => Lang::Es,
        _ => Lang::En,
    };
    let _ = LANG.set(locale);
}

pub fn get_language() -> &'static str {
    match lang() {
        Lang::En => "en",
        Lang::De => "de",
        Lang::Ru => "ru",
        Lang::Zh => "zh",
        Lang::Es => "es",
    }
}

fn lang() -> &'static Lang {
    LANG.get().unwrap_or(&Lang::En)
}

macro_rules! tr {
    ($en:expr, $de:expr, $ru:expr, $zh:expr, $es:expr) => {
        match lang() {
            Lang::En => $en,
            Lang::De => $de,
            Lang::Ru => $ru,
            Lang::Zh => $zh,
            Lang::Es => $es,
        }
    };
}

// ── UI элементы ───────────────────────────────────────────────────────────────

pub fn toolbar_open()       -> &'static str { tr!("Open", "Öffnen", "Открыть", "打开", "Abrir") }
pub fn toolbar_magnet()     -> &'static str { tr!("Magnet", "Magnet", "Magnet", "磁力", "Magnet") }
pub fn toolbar_create()     -> &'static str { tr!("Create", "Erstellen", "Создать", "创建", "Crear") }
pub fn toolbar_rehash()     -> &'static str { tr!("Rehash", "Prüfen", "Rehash", "校验", "Rehash") }

pub fn sidebar_status()     -> &'static str { tr!("Status", "Status", "СОСТОЯНИЕ", "状态", "Estado") }
pub fn sidebar_all()        -> &'static str { tr!("All", "Alle", "Все", "全部", "Todos") }
pub fn sidebar_downloading()-> &'static str { tr!("Downloading", "Laden", "Загружаются", "下载中", "Descargando") }
pub fn sidebar_seeding()    -> &'static str { tr!("Seeding", "Seeden", "Раздаются", "做种中", "Sembrando") }
pub fn sidebar_completed()  -> &'static str { tr!("Completed", "Fertig", "Завершены", "已完成", "Completados") }
pub fn sidebar_stopped()    -> &'static str { tr!("Stopped", "Gestoppt", "Остановлены", "已停止", "Detenidos") }
pub fn sidebar_active()     -> &'static str { tr!("Active", "Aktiv", "Активные", "活动中", "Activos") }
pub fn sidebar_inactive()   -> &'static str { tr!("Inactive", "Inaktiv", "Неактивные", "不活动", "Inactivos") }
pub fn sidebar_checking()   -> &'static str { tr!("Checking", "Prüfe", "Проверяются", "校验中", "Verificando") }
pub fn sidebar_error()      -> &'static str { tr!("Error", "Fehler", "Ошибка", "错误", "Error") }
pub fn sidebar_disks()      -> &'static str { tr!("Disks", "Laufwerke", "ДИСКИ", "磁盘", "Discos") }

pub fn column_name()        -> &'static str { tr!("Name", "Name", "Имя", "名称", "Nombre") }
pub fn column_status()      -> &'static str { tr!("Status", "Status", "Статус", "状态", "Estado") }
pub fn column_done()        -> &'static str { tr!("Done", "Fertig", "Готово", "完成", "Hecho") }
pub fn column_down()        -> &'static str { tr!("Down", "Runter", "Загрузка", "下载", "Bajada") }
pub fn column_up()          -> &'static str { tr!("Up", "Hoch", "Отдача", "上传", "Subida") }
pub fn column_actions()     -> &'static str { tr!("Actions", "Aktionen", "Действия", "操作", "Acciones") }

pub fn status_connected()   -> &'static str { tr!("Connected", "Verbunden", "Подключено", "已连接", "Conectado") }
pub fn status_disconnected()-> &'static str { tr!("Disconnected", "Getrennt", "Отключено", "未连接", "Desconectado") }
pub fn status_active()      -> &'static str { tr!("active", "aktiv", "активных", "活动", "activos") }

pub fn dlg_ok()             -> &'static str { tr!("OK", "OK", "OK", "确定", "OK") }
pub fn dlg_cancel()         -> &'static str { tr!("Cancel", "Abbrechen", "Отмена", "取消", "Cancelar") }
pub fn dlg_browse()         -> &'static str { tr!("Browse…", "Durchsuchen…", "Обзор…", "浏览…", "Examinar…") }

pub fn settings_language()  -> &'static str { tr!("Language", "Sprache", "Язык", "语言", "Idioma") }
pub fn settings_restart()   -> &'static str { tr!("⚠ Restart required", "⚠ Neustart erforderlich", "⚠ Требуется перезапуск", "⚠ 需要重启", "⚠ Se requiere reinicio") }

// ── Tray меню ─────────────────────────────────────────────────────────────────

pub fn tray_show_hide()   -> &'static str { tr!("Show / Hide", "Anzeigen / Ausblenden", "Показать / Скрыть", "显示 / 隐藏", "Mostrar / Ocultar") }
pub fn tray_resume_all()  -> &'static str { tr!("Resume All", "Alle fortsetzen", "Запустить все", "全部恢复", "Reanudar todos") }
pub fn tray_pause_all()   -> &'static str { tr!("Pause All", "Alle anhalten", "Остановить все", "全部暂停", "Pausar todos") }
pub fn tray_quit()        -> &'static str { tr!("Quit", "Beenden", "Выход", "退出", "Salir") }

// ── Статусы торрентов (из Transmission) ──────────────────────────────────────

pub fn status_stopped()     -> &'static str { tr!("Stopped", "Gestoppt", "Приостановлено", "已停止", "Detenido") }
pub fn status_check_wait()  -> &'static str { tr!("Waiting to check", "Warte auf Prüfung", "Очередь проверки", "等待校验", "Esperando verificación") }
pub fn status_checking()    -> &'static str { tr!("Checking", "Prüfe", "Проверка", "校验中", "Verificando") }
pub fn status_dl_queue()    -> &'static str { tr!("Queued", "Warteschlange", "Очередь загрузки", "下载队列", "En cola") }
pub fn status_downloading() -> &'static str { tr!("Downloading", "Lade herunter", "Загрузка", "下载中", "Descargando") }
pub fn status_seed_queue()  -> &'static str { tr!("Queued for seeding", "Warteschlange", "Очередь раздачи", "做种队列", "En cola para seed") }
pub fn status_seeding()     -> &'static str { tr!("Seeding", "Seede", "Раздача", "做种中", "Sembrando") }
pub fn status_unknown()     -> &'static str { tr!("Unknown", "Unbekannt", "Неизвестно", "未知", "Desconocido") }

// ── Ошибки ────────────────────────────────────────────────────────────────────

pub fn err_tracker_warn()   -> &'static str { tr!("Tracker warning", "Tracker Warnung", "Предупреждение трекера", "Tracker 警告", "Advertencia tracker") }
pub fn err_tracker_err()    -> &'static str { tr!("Tracker error", "Tracker Fehler", "Ошибка трекера", "Tracker 错误", "Error tracker") }
pub fn err_missing()        -> &'static str { tr!("No data", "Keine Daten", "Нет данных", "无数据", "Sin datos") }
pub fn err_generic()        -> &'static str { tr!("Error", "Fehler", "Ошибка", "错误", "Error") }
pub fn err_hash_fail()      -> &'static str { tr!("Hash error", "Hash Fehler", "Hash не совпадает", "Hash 错误", "Error hash") }

// ── Диалог удаления ──────────────────────────────────────────────────────────

pub fn dlg_remove_confirm() -> &'static str { tr!("Remove torrent?", "Torrent entfernen?", "Удалить торрент?", "移除种子?", "¿Eliminar torrent?") }
pub fn dlg_remove_subtitle()-> &'static str { tr!("(files will be kept on disk)", "(Dateien bleiben erhalten)", "(файлы останутся на диске)", "(文件将保留在磁盘上)", "(los archivos se mantendrán en el disco)") }
pub fn dlg_delete_confirm() -> &'static str { tr!("Delete torrent AND files?", "Torrent UND Dateien löschen?", "Удалить торрент и файлы?", "删除种子和文件?", "¿Eliminar torrent y archivos?") }
pub fn dlg_delete_warning() -> &'static str { tr!("All files will be permanently deleted from disk.", "Alle Dateien werden endgültig gelöscht.", "Все файлы будут безвозвратно удалены с диска.", "所有文件将从磁盘中永久删除。", "Todos los archivos serán eliminados permanentemente del disco.") }
pub fn dlg_irreversible()   -> &'static str { tr!("This action cannot be undone.", "Diese Aktion kann nicht rückgängig gemacht werden.", "Это действие нельзя отменить.", "此操作无法撤销。", "Esta acción no se puede deshacer.") }
