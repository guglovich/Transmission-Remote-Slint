// src/wm_icon.rs — устанавливает _NET_WM_ICON через X11 xcb
// PNG встроен в бинарник через include_bytes!
// XID окна получаем через xdotool по PID процесса
// Вызывается при старте и при каждом восстановлении из трея

static ICON_PNG: &[u8] = include_bytes!("../ui/app-icon.png");

pub fn set_wm_icon_by_pid() {
    std::thread::spawn(|| {
        let pid = std::process::id();
        for attempt in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(if attempt == 0 { 800 } else { 300 }));
            let out = std::process::Command::new("xdotool")
                .args(["search", "--pid", &pid.to_string(), "--onlyvisible"])
                .output();
            let xid = match out {
                Ok(o) if o.status.success() => {
                    let s = String::from_utf8_lossy(&o.stdout);
                    s.lines().next().and_then(|l| l.trim().parse::<u32>().ok())
                }
                _ => None,
            };
            if let Some(xid) = xid {
                match decode_and_set(xid) {
                    Ok(_) => { eprintln!("[wm_icon] Set _NET_WM_ICON for xid={xid}"); return; }
                    Err(e) => { eprintln!("[wm_icon] Failed for xid={xid}: {e}"); }
                }
            }
            eprintln!("[wm_icon] Window not found, retry {}...", attempt + 1);
        }
        eprintln!("[wm_icon] Could not find window XID after retries");
    });
}

fn decode_and_set(xid: u32) -> anyhow::Result<()> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::*;
    use x11rb::rust_connection::RustConnection;

    let img = image::load_from_memory(ICON_PNG)?.to_rgba8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        anyhow::bail!("app-icon.png has zero dimensions ({w}x{h})");
    }

    let mut data: Vec<u32> = Vec::new();

    data.push(w);
    data.push(h);
    for pixel in img.pixels() {
        let [r, g, b, a] = pixel.0;
        data.push(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32));
    }

    let small = image::imageops::resize(&img, 48, 48, image::imageops::FilterType::Lanczos3);
    data.push(48u32);
    data.push(48u32);
    for pixel in small.pixels() {
        let [r, g, b, a] = pixel.0;
        data.push(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32));
    }

    let (conn, _) = RustConnection::connect(None)?;
    let net_wm_icon = intern_atom(&conn, false, b"_NET_WM_ICON")?.reply()?.atom;
    let cardinal = intern_atom(&conn, false, b"CARDINAL")?.reply()?.atom;

    let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_ne_bytes()).collect();
    let num_items = (bytes.len() / 4) as u32;

    change_property(
        &conn,
        PropMode::REPLACE,
        xid,
        net_wm_icon,
        cardinal,
        32,
        num_items,
        &bytes,
    )?.check()?;

    conn.flush()?;
    Ok(())
}
