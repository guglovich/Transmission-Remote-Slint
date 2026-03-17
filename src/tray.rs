// src/tray.rs — ksni 0.3.3, StatusNotifierItem без GTK

use std::sync::mpsc;

#[derive(Debug)]
pub enum TrayCmd {
    ToggleWindow,
    Quit,
}

#[derive(Debug)]
struct TransmissionTray {
    tx: mpsc::SyncSender<TrayCmd>,
}

impl ksni::Tray for TransmissionTray {
    // В XFCE левый клик по умолчанию вызывает activate().
    // Ставим MENU_ON_ACTIVATE = true → левый клик открывает меню,
    // как принято в большинстве системных апплетов.
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String { "transmission-gui".into() }
    fn title(&self) -> String { "Transmission Remote".into() }

    fn category(&self) -> ksni::Category {
        ksni::Category::ApplicationStatus
    }

    fn status(&self) -> ksni::Status {
        ksni::Status::Active
    }

    // Системная иконка — DE найдёт из hicolor/Papirus/etc.
    fn icon_name(&self) -> String { "transmission".into() }

    // Fallback иконка 22×22 ARGB если системная не найдена
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let size: i32 = 22;
        let (cx, cy) = (11.0f32, 11.0f32);
        let r = 10.0f32;
        let mut data = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let dist = (((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)) as f32).sqrt();
                let alpha = if dist <= r - 1.0 { 255u8 }
                            else if dist <= r { ((r - dist) * 255.0) as u8 }
                            else { 0u8 };
                if alpha > 0 {
                    let t = (dist / r).min(1.0);
                    let px: u32 = ((alpha as u32) << 24)
                                | (((20.0 + 30.0*t) as u32) << 16)
                                | (((80.0 + 80.0*t) as u32) << 8)
                                | (200.0 - 30.0*t) as u32;
                    data.extend_from_slice(&px.to_be_bytes());
                } else {
                    data.extend_from_slice(&[0u8; 4]);
                }
            }
        }
        vec![ksni::Icon { width: size, height: size, data }]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: "transmission".into(),
            icon_pixmap: vec![],
            title: "Transmission Remote".into(),
            description: "BitTorrent client".into(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        eprintln!("[tray] activate() called");
        let _ = self.tx.send(TrayCmd::ToggleWindow);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Show / Hide".into(),
                icon_name: "window-restore".into(),
                activate: Box::new(|t: &mut Self| {
                    eprintln!("[tray] menu: Show/Hide");
                    let _ = t.tx.send(TrayCmd::ToggleWindow);
                }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|t: &mut Self| {
                    eprintln!("[tray] menu: Quit");
                    let _ = t.tx.send(TrayCmd::Quit);
                }),
                ..Default::default()
            }.into(),
        ]
    }

    // Если StatusNotifierWatcher пропал (DE перезапустился) — пробуем переподключиться
    fn watcher_offline(&self, reason: ksni::OfflineReason) -> bool {
        eprintln!("[tray] Watcher offline: {reason:?} — will retry");
        true // true = пробовать переподключиться
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub struct AppTray {
    rx: mpsc::Receiver<TrayCmd>,
    _handle: ksni::Handle<TransmissionTray>,
}

impl AppTray {
    pub fn build(rt: &tokio::runtime::Runtime) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::sync_channel::<TrayCmd>(8);
        use ksni::TrayMethods;
        let handle = rt.block_on(async {
            TransmissionTray { tx }.spawn().await
        }).map_err(|e| anyhow::anyhow!("tray spawn: {e}"))?;
        Ok(Self { rx, _handle: handle })
    }

    pub fn poll_events(&self) -> (bool, bool) {
        let (mut toggle, mut quit) = (false, false);
        while let Ok(cmd) = self.rx.try_recv() {
            eprintln!("[tray] poll_events got: {:?}", cmd);
            match cmd {
                TrayCmd::ToggleWindow => toggle = true,
                TrayCmd::Quit        => quit   = true,
            }
        }
        (toggle, quit)
    }
}
