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
pub fn menu_start()         -> &'static str { tr!("Start", "Starten", "Запустить", "开始", "Iniciar") }
pub fn menu_pause()         -> &'static str { tr!("Pause", "Anhalten", "Пауза", "暂停", "Pausar") }
pub fn menu_recheck()       -> &'static str { tr!("Recheck", "Neu prüfen", "Перепроверить", "重新校验", "Verificar de nuevo") }
pub fn menu_open_folder()   -> &'static str { tr!("Open Folder", "Ordner öffnen", "Открыть папку", "打开文件夹", "Abrir carpeta") }
pub fn menu_set_location()  -> &'static str { tr!("Set Location", "Speicherort festlegen", "Указать расположение", "设置位置", "Establecer ubicación") }
pub fn menu_remove()        -> &'static str { tr!("Remove Torrent", "Torrent entfernen", "Удалить торрент", "移除种子", "Eliminar torrent") }
pub fn menu_delete()        -> &'static str { tr!("Delete Torrent + Files", "Torrent + Dateien löschen", "Удалить торрент и файлы", "删除种子和文件", "Eliminar torrent y archivos") }

pub fn status_connected()   -> &'static str { tr!("Connected", "Verbunden", "Подключено", "已连接", "Conectado") }
pub fn status_disconnected()-> &'static str { tr!("Disconnected", "Getrennt", "Отключено", "未连接", "Desconectado") }
pub fn status_active()      -> &'static str { tr!("active", "aktiv", "активных", "活动", "activos") }

pub fn dlg_ok()             -> &'static str { tr!("OK", "OK", "OK", "确定", "OK") }
pub fn dlg_cancel()         -> &'static str { tr!("Cancel", "Abbrechen", "Отмена", "取消", "Cancelar") }
pub fn dlg_browse()         -> &'static str { tr!("Browse…", "Durchsuchen…", "Обзор…", "浏览…", "Examinar…") }

pub fn settings_language() -> &'static str { tr!("Language", "Sprache", "Язык", "语言", "Idioma") }
pub fn settings_title() -> &'static str { tr!("Settings", "Einstellungen", "Настройки", "设置", "Configuración") }
pub fn settings_restart() -> &'static str { tr!("⚠ Restart required", "⚠ Neustart erforderlich", "⚠ Требуется перезапуск", "⚠ 需要重启", "⚠ Se requiere reinicio") }
pub fn settings_autostart() -> &'static str { tr!("Autostart", "Autostart", "Автозапуск", "自动启动", "Autoinicio") }
pub fn settings_suspend() -> &'static str { tr!("Suspend when hidden", "Anhalten wenn versteckt", "Заморозить при скрытии", "隐藏时挂起", "Suspender al ocultar") }
pub fn settings_start_minimized() -> &'static str { tr!("Start minimized", "Minimiert starten", "Запускать свёрнутым", "最小化启动", "Iniciar minimizado") }
pub fn settings_delete_torrent() -> &'static str { tr!("Delete .torrent after adding", ".torrent nach Hinzufügen löschen", "Удалять .torrent после добавления", "添加后删除 .torrent", "Eliminar .torrent tras añadir") }
    pub fn settings_refresh_interval() -> &'static str { tr!("Refresh interval (sec)", "Aktualisierungsintervall (Sek)", "Интервал обновления (сек)", "刷新间隔（秒）", "Intervalo de actualización (seg)") }
    pub fn settings_save() -> &'static str { tr!("Save", "Speichern", "Сохранить", "保存", "Guardar") }
    pub fn settings_tab_system() -> &'static str { tr!("System", "System", "Система", "系统", "Sistema") }
    pub fn settings_tab_ui() -> &'static str { tr!("Interface", "Oberfläche", "Интерфейс", "界面", "Interfaz") }
    pub fn settings_tab_about() -> &'static str { tr!("About", "Über", "О программе", "关于", "Acerca de") }
    pub fn settings_on_close() -> &'static str { tr!("On window close:", "Beim Schließen:", "При закрытии окна:", "关闭窗口时：", "Al cerrar ventana:") }
    pub fn settings_on_close_ask() -> &'static str { tr!("Ask + wait trackers", "Fragen + Tracker warten", "Спрашивать + ждать трекеры", "询问 + 等待 Tracker", "Preguntar + esperar trackers") }
    pub fn settings_on_close_tray() -> &'static str { tr!("Minimize to tray", "In Tray minimieren", "Сворачивать в трей", "最小化到托盘", "Minimizar a bandeja") }
    pub fn settings_on_close_quit() -> &'static str { tr!("Quit immediately", "Sofort beenden", "Закрывать сразу", "立即退出", "Salir inmediatamente") }
    pub fn settings_notifications() -> &'static str { tr!("Notifications", "Benachrichtigungen", "Уведомления", "通知", "Notificaciones") }
    pub fn settings_notify_add() -> &'static str { tr!("Notify after adding torrents", "Nach Hinzufügen benachrichtigen", "Уведомление после добавления торрентов", "添加种子后通知", "Notificar al añadir torrents") }
    pub fn settings_notify_complete() -> &'static str { tr!("Notify after download complete", "Nach Download benachrichtigen", "Уведомление после завершения загрузки", "下载完成后通知", "Notificar al completar descarga") }
    pub fn settings_notify_sound() -> &'static str { tr!("Play sound on completion", "Ton bei Abschluss", "Звук при завершении", "完成时播放声音", "Sonido al completar") }
    pub fn settings_theme() -> &'static str { tr!("Theme:", "Thema:", "Тема:", "主题：", "Tema:") }
    pub fn settings_theme_dark() -> &'static str { tr!("Dark (default)", "Dunkel (Standard)", "Тёмная (по умолчанию)", "深色（默认）", "Oscuro (predeterminado)") }
    pub fn settings_theme_light() -> &'static str { tr!("Light", "Hell", "Светлая", "浅色", "Claro") }
    pub fn settings_theme_system() -> &'static str { tr!("System", "System", "Системная", "系统", "Sistema") }
    pub fn settings_toolbar_buttons() -> &'static str { tr!("Toolbar buttons", "Symbolleiste", "Кнопки панели инструментов", "工具栏按钮", "Botones de barra") }
    pub fn settings_toolbar_hint() -> &'static str { tr!("Check to show. Drag to reorder.", "Ankreuzen zum Anzeigen. Ziehen zum Sortieren.", "Отметьте нужные. Зажмите для смены порядка.", "勾选显示。拖动排序。", "Marque para mostrar. Arrastre para reordenar.") }
    pub fn settings_left_panel_sections() -> &'static str { tr!("Left panel sections", "Linke Panel-Sektionen", "Секции левой панели", "左侧面板分区", "Secciones del panel izquierdo") }
    pub fn settings_tb_add() -> &'static str { tr!("Add", "Hinzufügen", "Добавить", "添加", "Añadir") }
    pub fn settings_tb_magnet() -> &'static str { tr!("magnet", "Magnet", "magnet", "magnet", "magnet") }
    pub fn settings_tb_create() -> &'static str { tr!("Create", "Erstellen", "Создать", "创建", "Crear") }
    pub fn settings_tb_rehash() -> &'static str { tr!("Rehash errors", "Rehash-Fehler", "Рехэш ошибок", "校验错误", "Rehash errores") }
    pub fn settings_tb_start_sel() -> &'static str { tr!("Start selected", "Ausgewählte starten", "Старт выбранных", "启动选中", "Iniciar seleccionados") }
    pub fn settings_tb_pause_sel() -> &'static str { tr!("Pause selected", "Ausgewählte pausieren", "Пауза выбранных", "暂停选中", "Pausar seleccionados") }
    pub fn settings_tb_start_all() -> &'static str { tr!("Start all", "Alle starten", "Старт всех", "全部启动", "Iniciar todos") }
    pub fn settings_tb_pause_all() -> &'static str { tr!("Pause all", "Alle pausieren", "Пауза всех", "全部暂停", "Pausar todos") }
    pub fn settings_lp_status() -> &'static str { tr!("Status", "Status", "Состояние", "状态", "Estado") }
    pub fn settings_lp_disks() -> &'static str { tr!("Disks", "Laufwerke", "Диски", "磁盘", "Discos") }
    pub fn settings_lp_trackers() -> &'static str { tr!("Trackers", "Tracker", "Трекеры", "Tracker", "Trackers") }
    pub fn settings_lp_webtorrents() -> &'static str { tr!("Web-torrents", "Web-Torrents", "Веб-торренты", "Web种子", "Web-torrents") }
    pub fn settings_lp_tags() -> &'static str { tr!("Tags", "Tags", "Теги", "标签", "Etiquetas") }
    pub fn settings_lp_created() -> &'static str { tr!("Created by me", "Von mir erstellt", "Созданные мной", "我创建的", "Creados por mí") }

// ── About tab ──────────────────────────────────────────────────────────────

pub fn settings_about_desc() -> &'static str {
    tr!(
        "Lightweight native GUI for Transmission daemon.\nNo GTK, no Qt — Skia/OpenGL/Vulkan rendering.",
        "Leichtes natives GUI für Transmission Daemon.\nKein GTK, kein Qt — Skia/OpenGL/Vulkan-Rendering.",
        "Лёгкий нативный GUI для Transmission daemon.\nБез GTK, без Qt — рендеринг через Skia/OpenGL/Vulkan.",
        "轻量级原生 Transmission daemon 图形界面。\n无 GTK，无 Qt — Skia/OpenGL/Vulkan 渲染。",
        "GUI nativo ligero para Transmission daemon.\nSin GTK, sin Qt — renderizado Skia/OpenGL/Vulkan."
    )
}
pub fn settings_about_licence() -> &'static str { tr!("Licence", "Lizenz", "Лицензия", "许可证", "Licencia") }
pub fn settings_about_language() -> &'static str { tr!("Language", "Sprache", "Язык", "语言", "Idioma") }
pub fn settings_about_ui_framework() -> &'static str { tr!("UI framework", "UI-Framework", "UI-фреймворк", "UI 框架", "Framework UI") }
pub fn settings_about_renderer() -> &'static str { tr!("Renderer", "Renderer", "Рендерер", "渲染器", "Renderizador") }
pub fn settings_about_tray() -> &'static str { tr!("Tray", "Tray", "Трей", "托盘", "Bandeja") }
pub fn settings_about_developed_with() -> &'static str { tr!("Developed with", "Entwickelt mit", "Разработано с", "开发工具", "Desarrollado con") }

// ── Settings: Speed tab ────────────────────────────────────────────────────

pub fn settings_tab_speed() -> &'static str { tr!("Speed", "Geschwindigkeit", "Скорость", "速度", "Velocidad") }
pub fn settings_tab_download() -> &'static str { tr!("Download", "Download", "Загрузка", "下载", "Descarga") }
pub fn settings_tab_seeding() -> &'static str { tr!("Seeding", "Seeden", "Раздача", "做种", "Sembrado") }
pub fn settings_tab_network() -> &'static str { tr!("Network", "Netzwerk", "Сеть", "网络", "Red") }
pub fn settings_tab_privacy() -> &'static str { tr!("Privacy", "Privatsphäre", "Приватность", "隐私", "Privacidad") }
pub fn settings_tab_remote() -> &'static str { tr!("Remote", "Remote", "Удалённый доступ", "远程", "Remoto") }
pub fn settings_speed_limits() -> &'static str { tr!("Speed limits", "Geschwindigkeitslimits", "Ограничения скорости", "速度限制", "Límites de velocidad") }
pub fn settings_speed_upload() -> &'static str { tr!("Upload (MB/s):", "Upload (MB/s):", "Отдача (МБ/с):", "上传 (MB/s):", "Subida (MB/s):") }
pub fn settings_speed_download() -> &'static str { tr!("Download (MB/s):", "Download (MB/s):", "Загрузка (МБ/с):", "下载 (MB/s):", "Descarga (MB/s):") }
pub fn settings_alt_speed_limits() -> &'static str { tr!("Alternative speed limits", "Alternative Geschwindigkeitslimits", "Особые ограничения скорости", "备用速度限制", "Límites alternativos") }
pub fn settings_alt_speed_override() -> &'static str { tr!("Override normal speed limits by schedule or manually", "Normale Limits nach Zeitplan oder manuell überschreiben", "Замещение обычных ограничений по расписанию или вручную", "按计划或手动覆盖正常速度限制", "Sobrescribir límites normales por horario o manualmente") }
pub fn settings_alt_speed_upload() -> &'static str { tr!("Upload (MB/s):", "Upload (MB/s):", "Отдача (МБ/с):", "上传 (MB/s):", "Subida (MB/s):") }
pub fn settings_alt_speed_download() -> &'static str { tr!("Download (MB/s):", "Download (MB/s):", "Загрузка (МБ/с):", "下载 (MB/s):", "Descarga (MB/s):") }
pub fn settings_alt_speed_schedule() -> &'static str { tr!("By schedule:", "Nach Zeitplan:", "По расписанию:", "按计划：", "Por horario:") }
pub fn settings_alt_speed_to() -> &'static str { tr!("to", "bis", "до", "至", "a") }
pub fn settings_speed_day_mon() -> &'static str { tr!("Mon", "Mo", "Пн", "一", "Lun") }
pub fn settings_speed_day_tue() -> &'static str { tr!("Tue", "Di", "Вт", "二", "Mar") }
pub fn settings_speed_day_wed() -> &'static str { tr!("Wed", "Mi", "Ср", "三", "Mié") }
pub fn settings_speed_day_thu() -> &'static str { tr!("Thu", "Do", "Чт", "四", "Jue") }
pub fn settings_speed_day_fri() -> &'static str { tr!("Fri", "Fr", "Пт", "五", "Vie") }
pub fn settings_speed_day_sat() -> &'static str { tr!("Sat", "Sa", "Сб", "六", "Sáb") }
pub fn settings_speed_day_sun() -> &'static str { tr!("Sun", "So", "Вс", "日", "Dom") }

// ── Settings: Download tab ─────────────────────────────────────────────────

pub fn settings_dl_adding() -> &'static str { tr!("Adding", "Hinzufügen", "Добавление", "添加", "Añadir") }
pub fn settings_dl_watch_dir() -> &'static str { tr!("Automatically add torrent files from:", "Torrents automatisch hinzufügen aus:", "Автоматически добавлять торрент-файлы из:", "自动从此处添加种子文件：", "Añadir torrents automáticamente desde:") }
pub fn settings_dl_show_dialog() -> &'static str { tr!("Show torrent options dialog", "Torrent-Optionen anzeigen", "Показывать диалог параметров торрента", "显示种子选项对话框", "Mostrar diálogo de opciones del torrent") }
pub fn settings_dl_start_added() -> &'static str { tr!("Start added torrents", "Hinzugefügte torrents starten", "Запускать добавленные торренты", "启动添加的种子", "Iniciar torrents añadidos") }
pub fn settings_dl_trash_torrent() -> &'static str { tr!("Move .torrent file to trash", ".torrent in den Papierkorb verschieben", "Убрать торрент-файл в корзину", "将 .torrent 文件移至回收站", "Mover .torrent a la papelera") }
pub fn settings_dl_default_dir() -> &'static str { tr!("Default folder:", "Standardordner:", "Папка по умолчанию:", "默认文件夹：", "Carpeta predeterminada:") }
pub fn settings_dl_queue() -> &'static str { tr!("Download queue", "Download-Warteschlange", "Очередь загрузки", "下载队列", "Cola de descarga") }
pub fn settings_dl_queue_max() -> &'static str { tr!("Maximum active downloads:", "Max. aktive Downloads:", "Максимум активных загрузок:", "最大活动下载数：", "Máximo de descargas activas:") }
pub fn settings_dl_seed_ratio_limit() -> &'static str { tr!("Seed ratio activity minutes:", "Seed-Ratio Aktivitätsminuten:", "Данные раздачи за N мин активности:", "做种比率活动分钟数：", "Minutos de actividad de ratio de seed:") }
pub fn settings_dl_process() -> &'static str { tr!("Download process", "Download-Prozess", "Процесс загрузки", "下载过程", "Proceso de descarga") }
pub fn settings_dl_part_ext() -> &'static str { tr!("Append «.part» to incomplete files", "«.part» an unvollständige Dateien anhängen", "Добавлять «.part» к именам незавершённых файлов", "为未完成文件添加「.part」后缀", "Añadir «.part» a archivos incompletos") }
pub fn settings_dl_incomplete_dir() -> &'static str { tr!("Save incomplete torrents to:", "Unvollständige torrents speichern in:", "Сохранять незавершённые торренты в:", "保存未完成种子到：", "Guardar torrents incompletos en:") }
pub fn settings_dl_done_script() -> &'static str { tr!("Run script on completion:", "Skript nach Abschluss ausführen:", "Выполнять сценарий по окончании загрузки:", "完成后运行脚本：", "Ejecutar script al completar:") }

// ── Settings: Seeding tab ──────────────────────────────────────────────────

pub fn settings_seed_limits() -> &'static str { tr!("Limits", "Limits", "Ограничения", "限制", "Límites") }
pub fn settings_seed_ratio() -> &'static str { tr!("Stop seeding at ratio:", "Seeden stoppen bei Ratio:", "Прекратить раздачу при рейтинге:", "达到比率时停止做种：", "Detener seed en ratio:") }
pub fn settings_seed_idle() -> &'static str { tr!("Stop seeding if idle for N min:", "Seeden stoppen bei N Min Inaktivität:", "Прекратить раздачу при простое N мин:", "空闲 N 分钟后停止做种：", "Detener seed si inactivo N min:") }
pub fn settings_seed_done_script() -> &'static str { tr!("Run script on seeding done:", "Skript nach Seeding ausführen:", "Выполнять сценарий по окончании раздачи:", "做种完成后运行脚本：", "Ejecutar script al finalizar seed:") }

// ── Settings: Network tab ──────────────────────────────────────────────────

pub fn settings_net_listening() -> &'static str { tr!("Listening port", "Eingehender Port", "Прослушивание порта", "监听端口", "Puerto de escucha") }
pub fn settings_net_port() -> &'static str { tr!("Incoming peer port:", "Eingehender Peer-Port:", "Порт входящих подключений:", "传入对等端口：", "Puerto entrante de pares:") }
pub fn settings_net_random_port() -> &'static str { tr!("Random port on startup", "Zufälliger Port beim Start", "Случайный порт при запуске", "启动时随机端口", "Puerto aleatorio al inicio") }
pub fn settings_net_forward_port() -> &'static str { tr!("Forward port (UPnP / NAT-PMP)", "Port weiterleiten (UPnP / NAT-PMP)", "Пробрасывать порт (UPnP / NAT-PMP)", "端口转发 (UPnP / NAT-PMP)", "Reenviar puerto (UPnP / NAT-PMP)") }
pub fn settings_net_peers() -> &'static str { tr!("Peer limits", "Peer-Limits", "Ограничения участников", "对等连接限制", "Límites de pares") }
pub fn settings_net_max_peers_torrent() -> &'static str { tr!("Max peers per torrent:", "Max. Peers pro Torrent:", "Макс. участников на торрент:", "每个种子最大对等数：", "Máx. pares por torrent:") }
pub fn settings_net_max_peers_total() -> &'static str { tr!("Max peers total:", "Max. Peers gesamt:", "Общий максимум участников:", "最大对等总数：", "Máx. pares total:") }
pub fn settings_net_options() -> &'static str { tr!("Options", "Optionen", "Параметры", "选项", "Opciones") }
pub fn settings_net_utp() -> &'static str { tr!("Use µTP for peer communication", "µTP für Peer-Kommunikation nutzen", "Использовать µTP для связи с участниками", "使用 µTP 进行对等通信", "Usar µTP para comunicación entre pares") }
pub fn settings_net_pex() -> &'static str { tr!("Use PEX for peer exchange", "PEX für Peer-Austausch nutzen", "Использовать PEX для обмена списками участников", "使用 PEX 进行对等交换", "Usar PEX para intercambio de pares") }
pub fn settings_net_dht() -> &'static str { tr!("Use DHT for peer discovery", "DHT für Peer-Entdeckung nutzen", "Использовать DHT для обнаружения участников", "使用 DHT 进行对等发现", "Usar DHT para descubrimiento de pares") }
pub fn settings_net_lpd() -> &'static str { tr!("Use local peer discovery (LPD)", "Lokale Peer-Entdeckung (LPD) nutzen", "Использовать локальное обнаружение участников (LPD)", "使用本地对等发现 (LPD)", "Usar descubrimiento local de pares (LPD)") }
pub fn settings_net_default_trackers() -> &'static str { tr!("Default public trackers", "Standard öffentliche Tracker", "Стандартные публичные трекеры", "默认公共 Tracker", "Trackers públicos predeterminados") }
pub fn settings_net_trackers_placeholder() -> &'static str { tr!("Add trackers, one per line…", "Tracker hinzufügen, einer pro Zeile…", "Добавьте трекеры, по одному на строку…", "添加 Tracker，每行一个…", "Añadir trackers, uno por línea…") }

// ── Settings: Privacy tab ──────────────────────────────────────────────────

pub fn settings_priv_encryption() -> &'static str { tr!("Encryption", "Verschlüsselung", "Шифрование", "加密", "Cifrado") }
pub fn settings_priv_encryption_mode() -> &'static str { tr!("Mode:", "Modus:", "Режим:", "模式：", "Modo:") }
pub fn settings_priv_preferred() -> &'static str { tr!("Prefer encryption", "Verschlüsselung bevorzugen", "Предпочитать шифрование", "优先加密", "Preferir cifrado") }
pub fn settings_priv_required() -> &'static str { tr!("Require encryption", "Verschlüsselung erforderlich", "Требовать шифрование", "要求加密", "Requerir cifrado") }
pub fn settings_priv_allow_incoming() -> &'static str { tr!("Allow unencrypted", "Unverschlüsselt erlauben", "Не шифровать", "允许未加密", "Permitir sin cifrar") }
pub fn settings_priv_blocklist() -> &'static str { tr!("Blocklist", "Blockliste", "Чёрный список IP", "黑名单", "Lista de bloqueo") }
pub fn settings_priv_blocklist_enabled() -> &'static str { tr!("Enable blocklist:", "Blockliste aktivieren:", "Включить чёрный список:", "启用黑名单：", "Habilitar lista de bloqueo:") }
pub fn settings_priv_blocklist_url() -> &'static str { tr!("URL:", "URL:", "URL:", "URL：", "URL:") }
pub fn settings_priv_blocklist_auto_update() -> &'static str { tr!("Allow automatic blocklist update", "Automatische Blocklisten-Aktualisierung erlauben", "Разрешить автоматическое обновление чёрного списка", "允许自动更新黑名单", "Permitir actualización automática de lista de bloqueo") }
pub fn settings_priv_blocklist_count() -> &'static str { tr!("0 entries in blocklist", "0 Einträge in Blockliste", "0 записей в чёрном списке", "黑名单中有 0 条记录", "0 entradas en lista de bloqueo") }

// ── Settings: Remote tab ──────────────────────────────────────────────────

pub fn settings_remote_title() -> &'static str { tr!("Remote access", "Fernzugriff", "Удалённое управление", "远程访问", "Acceso remoto") }
pub fn settings_remote_enabled() -> &'static str { tr!("Enable remote access", "Fernzugriff aktivieren", "Разрешить удалённый доступ", "启用远程访问", "Habilitar acceso remoto") }
pub fn settings_remote_port() -> &'static str { tr!("HTTP port:", "HTTP-Port:", "Порт HTTP:", "HTTP 端口：", "Puerto HTTP:") }
pub fn settings_remote_auth() -> &'static str { tr!("Require authentication", "Authentifizierung erforderlich", "Использовать аутентификацию", "需要身份验证", "Requerir autenticación") }
pub fn settings_remote_username() -> &'static str { tr!("Username:", "Benutzername:", "Имя пользователя:", "用户名：", "Usuario:") }
pub fn settings_remote_password() -> &'static str { tr!("Password:", "Passwort:", "Пароль:", "密码：", "Contraseña:") }
pub fn settings_remote_whitelist() -> &'static str { tr!("Only allow these IP addresses:", "Nur diese IP-Adressen erlauben:", "Разрешить доступ только данным IP-адресам:", "仅允许以下 IP 地址：", "Solo permitir estas direcciones IP:") }
pub fn settings_remote_whitelist_addr() -> &'static str { tr!("Addresses:", "Adressen:", "Адреса:", "地址：", "Direcciones:") }
pub fn settings_remote_add() -> &'static str { tr!("Add", "Hinzufügen", "Добавить", "添加", "Añadir") }
pub fn settings_remote_remove() -> &'static str { tr!("Remove", "Entfernen", "Удалить", "移除", "Eliminar") }
pub fn settings_coming_soon() -> &'static str { tr!("Coming in a future update", "Kommt in einem zukünftigen Update", "Ожидайте в будущем обновлении", "敬请期待后续更新", "Próximamente en una futura actualización") }

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

// ── Статусбар ─────────────────────────────────────────────────────────────────

pub fn statusbar_dht()  -> &'static str { tr!("DHT", "DHT", "DHT", "DHT", "DHT") }
pub fn statusbar_conn() -> &'static str { tr!("conn", "Verb.", "подкл.", "连接", "conex.") }

// ── Dialog/UI strings (5 языков) — автогенерация из app.slint инвентаря ──
pub fn d_about_desc() -> &'static str { tr!("Lightweight native GUI for Transmission daemon.
No GTK, no Qt — Skia/OpenGL rendering", "Leichtgewichtige native GUI für Transmission.
Ohne GTK/Qt — Rendering via Skia/OpenGL", "Лёгкий нативный GUI для Transmission daemon.
Без GTK, без Qt — рендеринг через Skia/OpenGL", "轻量级 Transmission 原生 GUI。
无 GTK/Qt — Skia/OpenGL 渲染", "GUI nativo ligero para Transmission.
Sin GTK/Qt — render Skia/OpenGL") }
pub fn d_about_title() -> &'static str { tr!("About Client", "Über den Client", "О программе", "关于客户端", "Acerca del cliente") }
pub fn d_add_connection() -> &'static str { tr!("Add Connection", "Verbindung hinzufügen", "Добавить", "添加连接", "Añadir conexión") }
pub fn d_alt_dl_lbl() -> &'static str { tr!("Download (Mbit/s):", "Download (Mbit/s):", "Загрузка (Мбит/с):", "下载 (Mbit/s)：", "Descarga (Mbit/s):") }
pub fn d_alt_override() -> &'static str { tr!("Override normal limits by schedule or manually", "Normale Limits per Zeitplan oder manuell übersteuern", "Замещение обычных лимитов по расписанию или вручную", "按计划或手动覆盖常规限制", "Anular límites normales por horario o manualmente") }
pub fn d_alt_up_lbl() -> &'static str { tr!("Upload (Mbit/s):", "Upload (Mbit/s):", "Отдача (Мбит/с):", "上传 (Mbit/s)：", "Subida (Mbit/s):") }
pub fn d_append_part() -> &'static str { tr!("Append «.part» to incomplete files names", "„.part“ an unvollständige Dateien anhängen", "Добавлять «.part» к именам незавершённых файлов", "为未完成文件名添加“.part”", "Añadir «.part» a archivos incompletos") }
pub fn d_at_add_url() -> &'static str { tr!("Add URL", "URL hinzufügen", "Добавить URL", "添加链接", "Añadir URL") }
pub fn d_at_browse() -> &'static str { tr!("📂 Browse .torrent…", "📂 .torrent wählen …", "📂 Выбрать .torrent…", "📂 选择 .torrent…", "📂 Buscar .torrent…") }
pub fn d_at_title() -> &'static str { tr!("Add torrent", "Torrent hinzufügen", "Добавить торрент", "添加种子", "Añadir torrent") }
pub fn d_at_url_ph() -> &'static str { tr!("magnet:?xt=urn:btih:…  or  https://…", "magnet:?xt=urn:btih:… oder https://…", "magnet:?xt=urn:btih:…  или  https://…", "magnet:?xt=urn:btih:… 或 https://…", "magnet:?xt=urn:btih:… o https://…") }
pub fn d_auto_import() -> &'static str { tr!("Automatically import torrents from folder:", "Torrents automatisch aus Ordner importieren:", "Автоматически добавлять торрент-файлы из:", "自动从文件夹导入种子：", "Importar torrents automáticamente de la carpeta:") }
pub fn d_autostart_lbl() -> &'static str { tr!("Start Transmission down on login", "Beim Login starten", "Автозапуск при входе в систему", "登录时启动", "Iniciar al entrar al sistema") }
pub fn d_blocklist_auto() -> &'static str { tr!("Allow automatic updates", "Automatische Updates erlauben", "Разрешить автоматическое обновление", "允许自动更新", "Permitir actualizaciones automáticas") }
pub fn d_blocklist_enable() -> &'static str { tr!("Enable blocklist:", "Blocklist aktivieren:", "Включить чёрный список:", "启用黑名单：", "Habilitar lista negra:") }
pub fn d_blocklist_entries() -> &'static str { tr!("Blocklist contains ", "Blocklist enthält ", "В чёрном списке ", "黑名单包含 ", "La lista contiene ") }
pub fn d_blocklist_entries2() -> &'static str { tr!(" records", " Einträge", " записей", " 条记录", " registros") }
pub fn d_blocklist_url() -> &'static str { tr!("Blocklist URL", "Blocklist-URL", "URL списка", "黑名单 URL", "URL de la lista") }
pub fn d_by_schedule() -> &'static str { tr!("By schedule:", "Nach Zeitplan:", "По расписанию:", "按计划：", "Por horario:") }
pub fn d_ca_done() -> &'static str { tr!("Daemon stopped", "Daemon gestoppt", "Демон остановлен", "守护进程已停止", "Demonio detenido") }
pub fn d_ca_done_closing() -> &'static str { tr!("Done. Closing application...", "Fertig. Anwendung wird geschlossen …", "Готово. Закрываем приложение...", "完成。正在关闭应用…", "Listo. Cerrando la aplicación…") }
pub fn d_ca_left() -> &'static str { tr!("Left to download: ", "Verbleibend: ", "Останется докачать: ", "剩余下载：", "Pendiente de descargar: ") }
pub fn d_ca_notifying() -> &'static str { tr!("Notifying trackers: ", "Benachrichtige Tracker: ", "Уведомление трекеров: ", "通知跟踪器：", "Notificando trackers: ") }
pub fn d_ca_sending() -> &'static str { tr!("Sending upload/download summary to trackers", "Sende Up-/Download-Zusammenfassung an Tracker", "Отправляется сводка отдачи/загрузки на трекеры", "正在向跟踪器发送上传/下载摘要", "Enviando resumen de subida/descarga a trackers") }
pub fn d_ca_shutting() -> &'static str { tr!("Shutting down...", "Herunterfahren …", "Завершение работы...", "正在关闭…", "Apagando…") }
pub fn d_ca_units() -> &'static str { tr!(" pcs", " Stk.", " шт...", " 个…", " uds…") }
pub fn d_cancel_btn() -> &'static str { tr!("Cancel", "Abbrechen", "Отмена", "取消", "Cancelar") }
pub fn d_cd_min_tray() -> &'static str { tr!("⬇ Minimize to tray", "⬇ In Tray minimieren", "⬇ Свернуть в трей", "⬇ 最小化到托盘", "⬇ Minimizar a bandeja") }
pub fn d_cd_quit() -> &'static str { tr!("✕ Quit", "✕ Beenden", "✕ Выход", "✕ 退出", "✕ Salir") }
pub fn d_cd_what() -> &'static str { tr!("What would you like to do?", "Was möchtest du tun?", "Что сделать?", "您想做什么？", "¿Qué quieres hacer?") }
pub fn d_check_port() -> &'static str { tr!("Check", "Prüfen", "Проверить порт", "检测", "Comprobar") }
pub fn d_config_found() -> &'static str { tr!("CONFIG FILE FOUND", "KONFIGDATEI GEFUNDEN", "КОНФИГ НАЙДЕН", "找到配置文件", "ARCHIVO DE CONFIG ENCONTRADO") }
pub fn d_config_not_found() -> &'static str { tr!("NO CONFIG FILE FOUND", "KEINE KONFIGDATEI GEFUNDEN", "КОНФИГ НЕ НАЙДЕН", "未找到配置文件", "NO HAY ARCHIVO DE CONFIG") }
pub fn d_conn_host() -> &'static str { tr!("Connecting to host…", "Verbinde mit Host …", "Подключение к хосту…", "正在连接主机…", "Conectando al host…") }
pub fn d_conn_profiles() -> &'static str { tr!("CONNECTION PROFILES", "VERBINDUNGSPROFILE", "ПРОФИЛИ ПОДКЛЮЧЕНИЙ", "连接配置", "PERFILES DE CONEXIÓN") }
pub fn d_ct_browse() -> &'static str { tr!("📂 Browse…", "📂 Auswählen …", "📂 Обзор…", "📂 浏览…", "📂 Examinar…") }
pub fn d_ct_create() -> &'static str { tr!("✚ Create", "✚ Erstellen", "✚ Создать", "✚ 创建", "✚ Crear") }
pub fn d_ct_path_ph() -> &'static str { tr!("/path/to/folder or file", "/pfad/zu/ordner oder datei", "/путь/к/папке или файлу", "/路径/到/文件夹或文件", "/ruta/a/carpeta o archivo") }
pub fn d_ct_title() -> &'static str { tr!("Create torrent", "Torrent erstellen", "Создать торрент", "创建种子", "Crear torrent") }
pub fn d_ct_trackers() -> &'static str { tr!("Trackers (one URL per line, optional)", "Tracker (eine URL pro Zeile, optional)", "Трекеры (по одному URL на строку, опционально)", "跟踪器（每行一个 URL，可选）", "Trackers (una URL por línea, opcional)") }
pub fn d_day_fr() -> &'static str { tr!("Fr", "Fr", "Пт", "五", "Vi") }
pub fn d_day_mo() -> &'static str { tr!("Mo", "Mo", "Пн", "一", "Lu") }
pub fn d_day_sa() -> &'static str { tr!("Sa", "Sa", "Сб", "六", "Sá") }
pub fn d_day_su() -> &'static str { tr!("Su", "So", "Вс", "日", "Do") }
pub fn d_day_th() -> &'static str { tr!("Th", "Do", "Чт", "四", "Ju") }
pub fn d_day_tu() -> &'static str { tr!("Tu", "Di", "Вт", "二", "Ma") }
pub fn d_day_we() -> &'static str { tr!("We", "Mi", "Ср", "三", "Mi") }
pub fn d_default_dir_lbl() -> &'static str { tr!("Default download location:", "Standard-Speicherort:", "Папка по умолчанию:", "默认下载位置：", "Ubicación de descarga predeterminada:") }
pub fn d_del_btn() -> &'static str { tr!("Delete", "Löschen", "Удал.", "删除", "Eliminar") }
pub fn d_del_confirm() -> &'static str { tr!("Are you sure you want to permanently delete", "Soll dies endgültig gelöscht werden", "Вы уверены, что хотите безвозвратно удалить", "确定要永久删除吗：", "¿Seguro que quieres eliminar permanentemente") }
pub fn d_del_profile_q() -> &'static str { tr!("Delete Profile?", "Profil löschen?", "Удалить профиль?", "删除配置？", "¿Eliminar perfil?") }
pub fn d_del_secs() -> &'static str { tr!("s", "s", "с", "秒", "s") }
pub fn d_del_word() -> &'static str { tr!("Delete", "Löschen", "Удалить", "删除", "Eliminar") }
pub fn d_dev_with() -> &'static str { tr!("Developed with", "Entwickelt mit", "Разработано с", "开发工具", "Desarrollado con") }
pub fn d_dht() -> &'static str { tr!("Use Distributed Hash Table (DHT)", "Distributed Hash Table (DHT) verwenden", "Использовать DHT для обнаружения участников", "使用 DHT 发现参与者", "Usar tabla hash distribuida (DHT)") }
pub fn d_dlimit_lbl() -> &'static str { tr!("Download limit (Mbit/s):", "Download-Limit (Mbit/s):", "Лимит загрузки (Мбит/с):", "下载限制 (Mbit/s)：", "Límite de descarga (Mbit/s):") }
pub fn d_done_script_dl() -> &'static str { tr!("Execute action when torrent is done:", "Aktion bei Torrent-Abschluss:", "Выполнять сценарий по окончании загрузки:", "种子完成时执行脚本：", "Ejecutar acción al completar torrent:") }
pub fn d_done_script_seed() -> &'static str { tr!("Call script when torrent is done seeding:", "Skript bei beendetem Seeding:", "Выполнять сценарий по окончании раздачи:", "做种完成时执行脚本：", "Ejecutar script al terminar siembra:") }
pub fn d_edit_btn() -> &'static str { tr!("Edit", "Bearbeiten", "Ред.", "编辑", "Editar") }
pub fn d_enc_disabled() -> &'static str { tr!("Disabled", "Deaktiviert", "Не шифровать", "不加密", "Desactivado") }
pub fn d_enc_prefer() -> &'static str { tr!("Prefer Encryption", "Verschlüsselung bevorzugen", "Предпочитать шифрование", "偏好加密", "Preferir cifrado") }
pub fn d_enc_require() -> &'static str { tr!("Require Encryption", "Verschlüsselung erzwingen", "Требовать шифрование", "强制加密", "Requerir cifrado") }
pub fn d_encryption_mode() -> &'static str { tr!("Encryption mode:", "Verschlüsselungsmodus:", "Режим:", "加密模式：", "Modo de cifrado:") }
pub fn d_enforce_auth() -> &'static str { tr!("Enforce authentication (password)", "Authentifizierung erzwingen (Passwort)", "Требовать аутентификацию (пароль)", "需要身份验证（密码）", "Requerir autenticación (contraseña)") }
pub fn d_enforce_auth_short() -> &'static str { tr!("Enforce authentication password", "Authentifizierung (Passwort)", "Требовать аутентификацию (пароль)", "需要身份验证（密码）", "Requerir autenticación (contraseña)") }
pub fn d_exit_now() -> &'static str { tr!("Exit now", "Jetzt beenden", "Выйти сейчас", "立即退出", "Salir ahora") }
pub fn d_free_disk_note() -> &'static str { tr!("* 1.76 TB free space on /dev/sda", "* 1,76 TB frei auf /dev/sda", "* 1.76 ТБ свободно на /dev/sda", "* /dev/sda 剩余 1.76 TB", "* 1,76 TB libres en /dev/sda") }
pub fn d_host_lbl() -> &'static str { tr!("IP address / Host:", "IP-Adresse / Host:", "IP-адрес / Хост:", "IP 地址 / 主机：", "Dirección IP / Host:") }
pub fn d_incomplete_dir_lbl() -> &'static str { tr!("Save incomplete torrents to:", "Unvollständige Torrents speichern unter:", "Сохранять незавершённые торренты в:", "未完成种子保存至：", "Guardar torrents incompletos en:") }
pub fn d_lang_lbl() -> &'static str { tr!("Language", "Sprache", "Язык", "语言", "Idioma") }
pub fn d_lang_section() -> &'static str { tr!("INTERFACE LANGUAGE", "OBERFLÄCHENSPRACHE", "ЯЗЫК ИНТЕРФЕЙСА", "界面语言", "IDIOMA DE LA INTERFAZ") }
pub fn d_license_lbl() -> &'static str { tr!("License", "Lizenz", "Лицензия", "许可证", "Licencia") }
pub fn d_local_host() -> &'static str { tr!("Local Host", "Lokaler Host", "Local Host", "本地主机", "Host local") }
pub fn d_login_ph() -> &'static str { tr!("Login", "Login", "Логин", "登录", "Usuario") }
pub fn d_lpd() -> &'static str { tr!("Use Local Peer Discovery (LPD)", "Lokale Peer-Erkennung (LPD) verwenden", "Использовать локальное обнаружение участников", "使用本地参与者发现", "Usar descubrimiento local de pares (LPD)") }
pub fn d_manage_profiles() -> &'static str { tr!("Manage Profiles…", "Profile verwalten …", "Управление профилями…", "管理配置…", "Gestionar perfiles…") }
pub fn d_mg_title() -> &'static str { tr!("Add Magnet Link", "Magnet-Link hinzufügen", "Добавить магнитную ссылку", "添加磁力链接", "Añadir enlace magnético") }
pub fn d_no_profiles() -> &'static str { tr!("No connection profiles. Add a host manually.", "Keine Verbindungsprofile. Host manuell hinzufügen.", "Нет профилей подключений. Добавьте хост вручную.", "暂无连接配置。请手动添加主机。", "Sin perfiles. Añade un host manualmente.") }
pub fn d_notify_add() -> &'static str { tr!("Show notification when adding torrents", "Beim Hinzufügen benachrichtigen", "Показывать уведомление после добавления торрентов", "添加种子时显示通知", "Notificar al añadir torrents") }
pub fn d_notify_complete() -> &'static str { tr!("Show notification when download completes", "Bei abgeschlossenem Download benachrichtigen", "Показывать уведомление после завершения загрузки", "下载完成时显示通知", "Notificar al completar descarga") }
pub fn d_notify_sound() -> &'static str { tr!("Play sound notification upon complete", "Ton bei Abschluss abspielen", "Воспроизводить звуковое уведомление при завершении", "完成时播放声音", "Reproducir sonido al completar") }
pub fn d_on_close_ask() -> &'static str { tr!("Ask + wait for trackers", "Fragen + auf Tracker warten", "Спрашивать + ждать трекеры", "询问 + 等待跟踪器", "Preguntar + esperar trackers") }
pub fn d_on_close_lbl() -> &'static str { tr!("On window close:", "Beim Schließen des Fensters:", "При закрытии окна:", "关闭窗口时：", "Al cerrar la ventana:") }
pub fn d_on_close_quit() -> &'static str { tr!("Close immediately", "Sofort schließen", "Закрывать немедленно", "立即关闭", "Cerrar inmediatamente") }
pub fn d_on_close_tray() -> &'static str { tr!("Minimize to tray", "In Tray minimieren", "Сворачивать в трей", "最小化到托盘", "Minimizar a bandeja") }
pub fn d_password_ph() -> &'static str { tr!("Password", "Passwort", "Пароль", "密码", "Contraseña") }
pub fn d_path_to_script() -> &'static str { tr!("path to script", "Pfad zum Skript", "путь к скрипту", "脚本路径", "ruta al script") }
pub fn d_peer_port_lbl() -> &'static str { tr!("Peer listening port:", "Peer-Listening-Port:", "Порт входящих подключений:", "传入连接端口：", "Puerto de escucha de pares:") }
pub fn d_peers_max_global() -> &'static str { tr!("Global total peers limit:", "Globales Peer-Limit:", "Общий максимум участников:", "全局参与者上限：", "Límite global de pares:") }
pub fn d_peers_max_torrent() -> &'static str { tr!("Max peers per torrent:", "Max. Peers pro Torrent:", "Макс. участников на торрент:", "每种子最大参与者：", "Máx. pares por torrent:") }
pub fn d_pex() -> &'static str { tr!("Use Peer Exchange (PEX)", "Peer Exchange (PEX) verwenden", "Использовать PEX для обмена списками участников", "使用 PEX 交换参与者列表", "Usar intercambio de pares (PEX)") }
pub fn d_port_checking() -> &'static str { tr!("checking...", "prüfe...", "проверка...", "检测中…", "comprobando…") }
pub fn d_port_checking_cap() -> &'static str { tr!("Checking...", "Prüfe …", "Проверка...", "检测中…", "Comprobando…") }
pub fn d_port_closed() -> &'static str { tr!("closed", "geschlossen", "закрыт", "关闭", "cerrado") }
pub fn d_port_open() -> &'static str { tr!("open", "offen", "открыт", "开放", "abierto") }
pub fn d_port_unknown() -> &'static str { tr!("unknown", "unbekannt", "неизвестно", "未知", "desconocido") }
pub fn d_prevent_sleep() -> &'static str { tr!("Prevent sleep mode when torrents are active", "Ruhezustand bei aktiven Torrents verhindern", "Запрещать переход в спящий режим при активных торрентах", "有活动种子时阻止休眠", "Evitar suspensión con torrents activos") }
pub fn d_profile_name() -> &'static str { tr!("Profile name:", "Profilname:", "Название профиля:", "配置名称：", "Nombre del perfil:") }
pub fn d_public_trackers_ph() -> &'static str { tr!("Add trackers here, one per line...", "Tracker hier zeilenweise hinzufügen …", "Добавьте трекеры, по одному на строку...", "在此添加跟踪器，每行一个…", "Añade trackers, uno por línea…") }
pub fn d_queue_enable() -> &'static str { tr!("Enable download queue", "Download-Warteschlange aktivieren", "Включить очередь загрузок", "启用下载队列", "Habilitar cola de descargas") }
pub fn d_queue_max_lbl() -> &'static str { tr!("Maximum concurrent downloads:", "Max. gleichzeitige Downloads:", "Максимум активных загрузок:", "最大并发下载数：", "Descargas simultáneas máx.:") }
pub fn d_random_port() -> &'static str { tr!("Randomize port number on program launch", "Port bei jedem Start neu würfeln", "Устанавливать случайный порт при каждом запуске", "每次启动时随机端口", "Puerto aleatorio en cada inicio") }
pub fn d_reload_note() -> &'static str { tr!("* Dynamic elements will require standard client reload to change fully", "Volle Änderung erfordert Client-Neustart", "* Для полной смены локали может потребоваться перезапуск программы", "动态元素需重启客户端才能完全切换", "Los elementos dinámicos requieren reiniciar el cliente") }
pub fn d_renderer_lbl() -> &'static str { tr!("Renderer", "Renderer", "Рендерер", "渲染器", "Renderizador") }
pub fn d_save_btn() -> &'static str { tr!("Save", "Speichern", "Сохранить", "保存", "Guardar") }
pub fn d_sched_to() -> &'static str { tr!("to", "bis", "до", "至", "a") }
pub fn d_sec_additions() -> &'static str { tr!("ADDITIONS PREFERENCES", "HINZUFÜGEN-PRÄFERENZEN", "ДОБАВЛЕНИЕ", "添加首选项", "PREFERENCIAS DE ADICIÓN") }
pub fn d_sec_alt_limits() -> &'static str { tr!("SPECIAL ALTERNATIVE LIMITS", "SPEZIELLE ALTERNATIVE LIMITS", "ОСОБЫЕ ОГРАНИЧЕНИЯ СКОРОСТИ", "特殊备用限制", "LÍMITES ALTERNATIVOS ESPECIALES") }
pub fn d_sec_blocklist_ip() -> &'static str { tr!("IP BLOCKLIST", "IP-BLOCKLIST", "ЧЁРНЫЙ СПИСОК IP", "IP 黑名单", "LISTA NEGRA DE IP") }
pub fn d_sec_dl_process() -> &'static str { tr!("DOWNLOAD PROCESS", "DOWNLOAD-VORGANG", "ПРОЦЕСС ЗАГРУЗКИ", "下载过程", "PROCESO DE DESCARGA") }
pub fn d_sec_encryption() -> &'static str { tr!("PROTOCOL ENCRYPTION", "PROTOKOLLVERSCHLÜSSELUNG", "ШИФРОВАНИЕ", "协议加密", "CIFRADO DE PROTOCOLO") }
pub fn d_sec_features() -> &'static str { tr!("FEATURES", "FUNKTIONEN", "ПАРАМЕТРЫ", "功能", "CARACTERÍSTICAS") }
pub fn d_sec_iface_lang() -> &'static str { tr!("INTERFACE LANGUAGE", "OBERFLÄCHENSPRACHE", "ЯЗЫК ИНТЕРФЕЙСА", "界面语言", "IDIOMA DE LA INTERFAZ") }
pub fn d_sec_iface_visual() -> &'static str { tr!("GENERAL VISUAL SETTINGS", "ALLGEMEINE ANZEIGEEINSTELLUNGEN", "ОБЩИЙ ВИД", "常规外观设置", "AJUSTES VISUALES GENERALES") }
pub fn d_sec_lbl() -> &'static str { tr!("sec", "Sek.", "сек", "秒", "seg") }
pub fn d_sec_network() -> &'static str { tr!("INCOMING LISTENING PORT", "EINGEHENDER LISTENING-PORT", "ПРОСЛУШИВАНИЕ ПОРТА", "传入监听端口", "PUERTO DE ESCUCHA ENTRANTE") }
pub fn d_sec_notifications() -> &'static str { tr!("NOTIFICATIONS", "BENACHRICHTIGUNGEN", "УВЕДОМЛЕНИЯ", "通知", "NOTIFICACIONES") }
pub fn d_sec_peers() -> &'static str { tr!("PEERS & CONNECTIONS BOUNDARIES", "PEER- & VERBINDUNGSGRENZEN", "ОГРАНИЧЕНИЯ УЧАСТНИКОВ", "参与者与连接限制", "LÍMITES DE PARES Y CONEXIONES") }
pub fn d_sec_privacy() -> &'static str { tr!("PRIVACY", "DATENSCHUTZ", "ПРИВАТНОСТЬ", "隐私", "PRIVACIDAD") }
pub fn d_sec_public_trackers() -> &'static str { tr!("DEFAULT PUBLIC TRACKERS", "STANDARD-PUBLIC-TRACKER", "СТАНДАРТНЫЕ ПУБЛИЧНЫЕ ТРЕКЕРЫ", "默认公共跟踪器", "TRACKERS PÚBLICOS PREDETERMINADOS") }
pub fn d_sec_queue() -> &'static str { tr!("ACTIVE DOWNLOADS QUEUE", "AKTIVE DOWNLOAD-WARTESCHLANGE", "ОЧЕРЕДЬ ЗАГРУЗКИ", "活动下载队列", "COLA DE DESCARGAS ACTIVAS") }
pub fn d_sec_remote() -> &'static str { tr!("REMOTE CONTROL SERVICES", "FERNSTEUERUNGSDIENSTE", "УДАЛЁННОЕ УПРАВЛЕНИЕ", "远程控制服务", "SERVICIOS DE CONTROL REMOTO") }
pub fn d_sec_seeding() -> &'static str { tr!("SEEDING BOUNDARIES", "SEEDING-GRENZEN", "ОГРАНИЧЕНИЯ РАЗДАЧИ", "做种限制", "LÍMITES DE SEMBRADO") }
pub fn d_sec_speed_limits() -> &'static str { tr!("SPEED LIMITS", "GESCHWINDIGKEITSLIMITS", "ОГРАНИЧЕНИЯ СКОРОСТИ", "速度限制", "LÍMITES DE VELOCIDAD") }
pub fn d_seed_idle_lbl() -> &'static str { tr!("Stop seeding if idle (minutes):", "Seeding stoppen bei Inaktivität (Min.):", "Прекратить раздачу при простое (мин):", "空闲时停止做种（分钟）：", "Detener siembra inactiva (min):") }
pub fn d_seed_ratio_lbl() -> &'static str { tr!("Stop seeding under ratio:", "Seeding stoppen bei Verhältnis:", "Прекратить раздачу при рейтинге:", "比率低于此值时停止做种：", "Detener siembra bajo ratio:") }
pub fn d_seed_ratio_min_lbl() -> &'static str { tr!("Seed ratio limit min:", "Seed-Verhältnis Min.:", "Данные раздачи за N мин активности:", "做种比率最小值：", "Mín. de ratio de siembra:") }
pub fn d_settings_title() -> &'static str { tr!("Settings", "Einstellungen", "Настройки", "设置", "Ajustes") }
pub fn d_show_opts_dlg() -> &'static str { tr!("Show torrent options dialog", "Torrent-Optionen anzeigen", "Показывать диалог параметров торрента", "显示种子选项对话框", "Mostrar diálogo de opciones del torrent") }
pub fn d_sl_pick() -> &'static str { tr!("📍 Set location", "📍 Ort festlegen", "📍 Указать расположение", "📍 指定位置", "📍 Establecer ubicación") }
pub fn d_start_added() -> &'static str { tr!("Instantly trigger downloads after import", "Downloads nach Import sofort starten", "Запускать добавленные торренты", "导入后立即开始下载", "Iniciar descargas tras importar") }
pub fn d_start_min_tray() -> &'static str { tr!("Start minimized to tray", "Minimiert in Tray starten", "Запускать свёрнутым в трей", "启动时最小化到托盘", "Iniciar minimizado en bandeja") }
pub fn d_status_lbl() -> &'static str { tr!("Status:", "Status:", "Состояние:", "状态：", "Estado:") }
pub fn d_sync_state() -> &'static str { tr!("Synchronizing session state and torrent listings…", "Synchronisiere Sitzung und Liste …", "Синхронизация состояния сессии и списка раздач…", "正在同步会话状态与种子列表…", "Sincronizando estado y lista de torrents…") }
pub fn d_tab_downloading() -> &'static str { tr!("Downloading", "Downloads", "Загрузка", "下载", "Descargas") }
pub fn d_tab_interface() -> &'static str { tr!("Interface", "Oberfläche", "Интерфейс", "界面", "Interfaz") }
pub fn d_tab_network() -> &'static str { tr!("Network", "Netzwerk", "Сеть", "网络", "Red") }
pub fn d_tab_privacy() -> &'static str { tr!("Privacy", "Datenschutz", "Приватность", "隐私", "Privacidad") }
pub fn d_tab_remote() -> &'static str { tr!("Remote", "Fernsteuerung", "Удалённо", "远程", "Remoto") }
pub fn d_tab_seeding() -> &'static str { tr!("Seeding", "Seeding", "Раздача", "做种", "Siembra") }
pub fn d_tab_speed() -> &'static str { tr!("Speed", "Geschwindigkeit", "Скорость", "速度", "Velocidad") }
pub fn d_tab_system() -> &'static str { tr!("System", "System", "Система", "系统", "Sistema") }
pub fn d_trash_torrents() -> &'static str { tr!("Safely trash imported .torrents files", "Importierte .torrents in den Papierkorb", "Убрать торрент-файл в корзину", "将导入的 .torrent 移入回收站", "Mover .torrents importados a la papelera") }
pub fn d_tray_lbl() -> &'static str { tr!("Tray", "Tray", "Трей", "托盘", "Bandeja") }
pub fn d_uifw_lbl() -> &'static str { tr!("UI-Framework", "UI-Framework", "UI-фреймворк", "UI 框架", "Framework UI") }
pub fn d_ulimit_lbl() -> &'static str { tr!("Upload limit (Mbit/s):", "Upload-Limit (Mbit/s):", "Лимит отдачи (Мбит/с):", "上传限制 (Mbit/s)：", "Límite de subida (Mbit/s):") }
pub fn d_update_freq() -> &'static str { tr!("GUI list update frequency:", "Listen-Aktualisierung:", "Обновление списка раздач:", "列表刷新频率：", "Frecuencia de actualización:") }
pub fn d_upnp() -> &'static str { tr!("Forward ports automatically using UPnP/NAT-PMP", "Ports automatisch per UPnP/NAT-PMP öffnen", "Использовать UPnP или NAT-PMP", "使用 UPnP/NAT-PMP 自动转发端口", "Abrir puertos automáticamente con UPnP/NAT-PMP") }
pub fn d_utp() -> &'static str { tr!("Enable Micro Transport Protocol (µTP)", "Micro Transport Protocol (µTP) aktivieren", "Использовать µTP для связи с участниками", "启用微传输协议 (µTP)", "Habilitar Micro Transport Protocol (µTP)") }
pub fn d_mg_ph() -> &'static str { tr!("magnet:?xt=urn:btih:...", "magnet:?xt=urn:btih:...", "magnet:?xt=urn:btih:...", "magnet:?xt=urn:btih:...", "magnet:?xt=urn:btih:...") }
