// src/tray.rs — StatusNotifierItem через zbus 4, без ksni
// Запускается в отдельном потоке со своим tokio runtime — не конфликтует со Slint

use std::sync::mpsc;
use zbus::interface;

#[derive(Debug, Clone)]
pub enum TrayCmd {
    ToggleWindow,
    Quit,
    StartAll,
    StopAll,
}

struct StatusNotifierItem {
    tx: mpsc::SyncSender<TrayCmd>,
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl StatusNotifierItem {
    #[zbus(property)]
    fn id(&self) -> &str { "transmission-remote-slint" }
    #[zbus(property)]
    fn title(&self) -> &str { "Transmission Remote" }
    #[zbus(property)]
    fn category(&self) -> &str { "ApplicationStatus" }
    #[zbus(property)]
    fn status(&self) -> &str { "Active" }
    #[zbus(property)]
    fn icon_name(&self) -> &str { "transmission" }
    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        // Встроенный PNG → ARGB для SNI протокола
        static ICON_PNG: &[u8] = include_bytes!("../ui/app-icon.png");
        let Ok(img) = image::load_from_memory(ICON_PNG) else { return vec![]; };
        // 22×22 для трея
        let small = image::imageops::resize(&img.to_rgba8(), 22, 22, image::imageops::FilterType::Lanczos3);
        let data: Vec<u8> = small.pixels().flat_map(|p| {
            let [r, g, b, a] = p.0;
            // SNI ожидает ARGB в network byte order (big-endian)
            let px: u32 = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            px.to_be_bytes()
        }).collect();
        vec![(22, 22, data)]
    }
    #[zbus(property)]
    fn overlay_icon_name(&self) -> &str { "" }
    #[zbus(property)]
    fn attention_icon_name(&self) -> &str { "" }
    #[zbus(property)]
    fn tool_tip(&self) -> (String, Vec<(i32, i32, Vec<u8>)>, String, String) {
        ("transmission".into(), vec![], "Transmission Remote".into(), "BitTorrent client".into())
    }
    #[zbus(property)]
    fn menu(&self) -> zbus::zvariant::ObjectPath<'_> {
        zbus::zvariant::ObjectPath::try_from("/Menu").unwrap()
    }
    #[zbus(property)]
    fn item_is_menu(&self) -> bool { false }

    fn activate(&self, _x: i32, _y: i32) {
        eprintln!("[tray] Activate → ToggleWindow");
        let _ = self.tx.send(TrayCmd::ToggleWindow);
    }
    fn secondary_activate(&self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayCmd::ToggleWindow);
    }
    fn scroll(&self, _delta: i32, _orientation: &str) {}
    fn context_menu(&self, _x: i32, _y: i32) {}
}

struct DbusMenu {
    tx: mpsc::SyncSender<TrayCmd>,
}

#[interface(name = "com.canonical.dbusmenu")]
impl DbusMenu {
    #[zbus(property)] fn version(&self) -> u32 { 4 }
    #[zbus(property)] fn text_direction(&self) -> &str { "ltr" }
    #[zbus(property)] fn status(&self) -> &str { "normal" }
    #[zbus(property)] fn icon_theme_path(&self) -> Vec<String> { vec![] }

    fn get_layout(
        &self, _parent_id: i32, _recursion_depth: i32, _property_names: Vec<String>,
    ) -> (u32, (i32, std::collections::HashMap<String, zbus::zvariant::Value<'_>>, Vec<zbus::zvariant::Value<'_>>)) {
        type MenuItem = (i32, std::collections::HashMap<String, zbus::zvariant::Value<'static>>, Vec<zbus::zvariant::Value<'static>>);
        fn mk(id: i32, label: &str, icon: &str) -> MenuItem {
            let mut p = std::collections::HashMap::<String, zbus::zvariant::Value<'static>>::new();
            p.insert("label".into(), zbus::zvariant::Value::from(label.to_owned()));
            p.insert("enabled".into(), zbus::zvariant::Value::from(true));
            p.insert("visible".into(), zbus::zvariant::Value::from(true));
            p.insert("icon-name".into(), zbus::zvariant::Value::from(icon.to_owned()));
            p.insert("type".into(), zbus::zvariant::Value::from("standard"));
            (id, p, vec![])
        }
        fn sep() -> MenuItem {
            let mut p = std::collections::HashMap::<String, zbus::zvariant::Value<'static>>::new();
            p.insert("type".into(), zbus::zvariant::Value::from("separator"));
            p.insert("visible".into(), zbus::zvariant::Value::from(true));
            (2i32, p, vec![])
        }
        let mut root = std::collections::HashMap::<String, zbus::zvariant::Value<'_>>::new();
        root.insert("children-display".into(), zbus::zvariant::Value::from("submenu"));
        (1u32, (0i32, root, vec![
            zbus::zvariant::Value::from(mk(1, crate::i18n::tray_show_hide(), "window-restore")),
            zbus::zvariant::Value::from(sep()),
            zbus::zvariant::Value::from(mk(4, crate::i18n::tray_resume_all(), "media-playback-start")),
            zbus::zvariant::Value::from(mk(5, crate::i18n::tray_pause_all(), "media-playback-pause")),
            zbus::zvariant::Value::from(sep()),
            zbus::zvariant::Value::from(mk(3, crate::i18n::tray_quit(), "application-exit")),
        ]))
    }

    fn get_group_properties(&self, ids: Vec<i32>, _names: Vec<String>)
        -> Vec<(i32, std::collections::HashMap<String, zbus::zvariant::Value<'_>>)>
    { ids.into_iter().map(|id| (id, std::collections::HashMap::new())).collect() }

    fn get_property(&self, _id: i32, _name: &str) -> zbus::zvariant::Value<'_> {
        zbus::zvariant::Value::from("")
    }

    fn event(&self, id: i32, event_id: &str, _data: zbus::zvariant::Value<'_>, _ts: u32) {
        if event_id == "clicked" {
            match id {
                1 => { eprintln!("[tray] menu: Show/Hide"); let _ = self.tx.send(TrayCmd::ToggleWindow); }
                3 => { eprintln!("[tray] menu: Quit");      let _ = self.tx.send(TrayCmd::Quit); }
                4 => { eprintln!("[tray] menu: Resume All"); let _ = self.tx.send(TrayCmd::StartAll); }
                5 => { eprintln!("[tray] menu: Pause All");  let _ = self.tx.send(TrayCmd::StopAll); }
                _ => {}
            }
        }
    }

    fn event_group(&self, events: Vec<(i32, String, zbus::zvariant::Value<'_>, u32)>) -> Vec<i32> {
        for (id, ev, data, ts) in events { self.event(id, &ev, data, ts); }
        vec![]
    }

    fn about_to_show(&self, _id: i32) -> bool { false }
    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) { (vec![], vec![]) }

    #[zbus(signal)]
    async fn items_properties_updated(
        ctxt: &zbus::SignalContext<'_>,
        updated: Vec<(i32, std::collections::HashMap<String, zbus::zvariant::Value<'_>>)>,
        removed: Vec<(i32, Vec<String>)>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn layout_updated(ctxt: &zbus::SignalContext<'_>, revision: u32, parent: i32) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn item_activation_requested(ctxt: &zbus::SignalContext<'_>, id: i32, timestamp: u32) -> zbus::Result<()>;
}

pub struct AppTray {
    rx: mpsc::Receiver<TrayCmd>,
}

impl AppTray {
    pub fn build() -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::sync_channel::<TrayCmd>(8);
        let tx2 = tx.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all().build()
            {
                Ok(r) => r,
                Err(e) => { eprintln!("[tray] runtime error: {e}"); return; }
            };
            rt.block_on(async move {
                if let Err(e) = run_tray(tx2).await {
                    eprintln!("[tray] Error: {e}");
                }
            });
        });
        Ok(Self { rx })
    }

    pub fn poll_events(&self) -> (bool, bool, bool, bool) {
        let (mut toggle, mut quit, mut start_all, mut stop_all) = (false, false, false, false);
        while let Ok(cmd) = self.rx.try_recv() {
            eprintln!("[tray] poll_events got: {:?}", cmd);
            match cmd {
                TrayCmd::ToggleWindow => toggle = true,
                TrayCmd::Quit        => quit   = true,
                TrayCmd::StartAll    => start_all = true,
                TrayCmd::StopAll     => stop_all  = true,
            }
        }
        (toggle, quit, start_all, stop_all)
    }
}

async fn run_tray(tx: mpsc::SyncSender<TrayCmd>) -> anyhow::Result<()> {
    let conn = zbus::Connection::session().await?;
    let pid  = std::process::id();
    let name = format!("org.kde.StatusNotifierItem-{pid}-1");

    conn.object_server().at("/StatusNotifierItem", StatusNotifierItem { tx: tx.clone() }).await?;
    conn.object_server().at("/Menu", DbusMenu { tx }).await?;
    conn.request_name(name.as_str()).await?;

    match conn.call_method(
        Some("org.kde.StatusNotifierWatcher"),
        "/StatusNotifierWatcher",
        Some("org.kde.StatusNotifierWatcher"),
        "RegisterStatusNotifierItem",
        &name,
    ).await {
        Ok(_)  => eprintln!("[tray] StatusNotifierItem registered"),
        Err(e) => eprintln!("[tray] Register failed: {e}"),
    }

    loop { tokio::time::sleep(std::time::Duration::from_secs(60)).await; }
}
