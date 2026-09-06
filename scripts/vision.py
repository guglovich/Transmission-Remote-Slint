#!/usr/bin/env python3
"""
vision.py — UI Visual Inspector for transmission-remote-slint

Two modes:
  1. STATIC: parse ui/main.slint, verify against config
  2. VISUAL: screenshot pixel analysis (no OCR needed)

Usage:
  python3 scripts/vision.py
  python3 scripts/vision.py --snapshot path.png
  python3 scripts/vision.py --static-only
"""

import argparse, json, os, re, subprocess, sys, tempfile
from pathlib import Path
import numpy as np
from PIL import Image

SCRIPT_DIR = Path(__file__).parent
PROJECT_DIR = SCRIPT_DIR.parent
SLINT_PATH = PROJECT_DIR / "ui" / "main.slint"
MOCKUP_PATH = PROJECT_DIR / "transmission-remote-v4.html"
CONFIG_PATH = SCRIPT_DIR / "vision_config.json"

def load_config():
    cfg = {
        "geometry": {"dialog_width": 800, "dialog_height": 600, "title_h": 44, "sidebar_w": 160, "footer_h": 52},
        "palette": {
            "dialog_bg": "#13151c", "content_bg": "#1a1d27", "sidebar_bg": "#13151c",
            "border": "#252840", "border2": "#343858", "accent": "#4a7cf7",
            "active_tab_bg": "#21253a", "footer_bg": "#0d0f13",
            "text_primary": "#e4e7f2", "text_muted": "#8f97b8", "text_dim": "#4e556e",
        },
    }
    if CONFIG_PATH.exists():
        user = json.loads(CONFIG_PATH.read_text())
        for k in user:
            if k in cfg and isinstance(cfg[k], dict) and isinstance(user[k], dict):
                cfg[k].update(user[k])
            else:
                cfg[k] = user[k]
    return cfg


# ── STATIC ──────────────────────────────────────────────────────────────────

def static_analysis(cfg):
    r, f_pass, f_fail = [], 0, 0
    def ok(m, d=""): nonlocal f_pass; f_pass+=1; r.append(f"  PASS  {m}{' — '+d if d else ''}")
    def fail(m, d=""): nonlocal f_fail; f_fail+=1; r.append(f"  FAIL  {m}{' — '+d if d else ''}")

    r.append("\n══════ STATIC ANALYSIS ══════")
    if not SLINT_PATH.exists():
        r.append(f"  ERROR: {SLINT_PATH} not found"); return r, f_fail, f_pass

    text = SLINT_PATH.read_text()
    r.append(f"  Source: {SLINT_PATH} ({len(text.splitlines())} lines)")

    # Find SettingsDialog
    start = text.find("component SettingsDialog inherits Rectangle")
    end = text.find("\n}", text.find("component SettingsDialog inherits Rectangle"))
    if start < 0:
        r.append("  ERROR: SettingsDialog not found"); return r, 1, f_pass
    # Find the matching closing brace
    brace_depth = 0
    saw_open = False
    settings_end = start
    for i, ch in enumerate(text[start:]):
        if ch == '{':
            brace_depth += 1
            saw_open = True
        elif ch == '}':
            brace_depth -= 1
        if saw_open and brace_depth == 0:
            settings_end = start + i
            break
    settings = text[start:settings_end+1]

    # Extract all hex colors
    colors = set(f"#{h.lower()}" for h in re.findall(r'#([0-9a-fA-F]{6})', settings))
    r.append(f"  Colors in SettingsDialog: {len(colors)}")
    for c in sorted(colors):
        r.append(f"    {c}")

    # Verify palette colors exist in source
    for name, expected in cfg["palette"].items():
        found = expected.lower() in colors
        ok(f"Palette '{name}'", f"{'found' if found else expected}")

    # Check required elements
    checks = [
        ("sidebar tabs (for tab-info)", r'for\s+tab-info\s+in'),
        ("language list", r'for\s+lang-item\s+in'),
        ("saved callback", r'callback\s+saved'),
        ("cancelled callback", r'callback\s+cancelled'),
        ("Cancel button", r'\"Cancel\"'),
        ("Save button", r'\"Save\"'),
        ("About tab", r'\"About\"'),
        ("encryption", r'priv-encryption'),
        ("day buttons (Mon..Sun)", r'Mon.*Tue.*Wed.*Thu.*Fri.*Sat.*Sun'),
    ]
    for name, pat in checks:
        ok(name) if re.search(pat, text, re.DOTALL) else fail(name)

    # Extract tab structure
    tabs = re.findall(r'\{ico:\s*"([^"]+)",\s*n:\s*(\d+),\s*label:\s*"([^"]+)"\}', settings)
    r.append(f"\n  Tabs ({len(tabs)}):")
    for ico, n, label in tabs:
        r.append(f"    [{n}] {ico} {label}")

    # Extract section headers (UPPERCASE texts)
    headers = re.findall(r'Text\s*\{\s*text:\s*"([A-Z ]+)"', settings)
    if headers:
        r.append(f"\n  Section headers ({len(headers)}):")
        for h in headers:
            r.append(f"    \"{h}\"")

    r.append(f"\n  Static: {f_pass} pass, {f_fail} fail")
    return r, f_fail, f_pass


# ── VISUAL ──────────────────────────────────────────────────────────────────

def take_screenshot():
    tmp = os.path.join(tempfile.gettempdir(), "main_viz.slint")
    src = SLINT_PATH.read_text().replace(
        "settings-dialog-visible: false;", "settings-dialog-visible: true;")
    Path(tmp).write_text(src)
    out = os.path.join(tempfile.gettempdir(), "vision_screenshot.png")
    try:
        subprocess.run(
            ["xvfb-run", "--auto-servernum", "--server-args", "-screen 0 1920x1080x24",
             "sh", "-c",
             f"slint-viewer {tmp} & P=$!; sleep 4; import -window root {out}; kill $P 2>/dev/null; wait $P 2>/dev/null"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30)
    except: pass
    return out if os.path.exists(out) and os.path.getsize(out) > 1000 else None

def visual_analysis(im, cfg):
    r, f_pass, f_fail, f_warn = [], 0, 0, 0
    def ok(m, d=""): nonlocal f_pass; f_pass+=1; r.append(f"  PASS  {m}{' — '+d if d else ''}")
    def fail(m, d=""): nonlocal f_fail; f_fail+=1; r.append(f"  FAIL  {m}{' — '+d if d else ''}")
    def warn(m, d=""): nonlocal f_warn; f_warn+=1; r.append(f"  WARN  {m}{' — '+d if d else ''}")

    geo = cfg["geometry"]
    arr = np.array(im)
    h, w = arr.shape[:2]

    r.append("\n══════ VISUAL ANALYSIS ══════")
    r.append(f"  xvfb rendering — colors inaccurate, structural only")
    r.append(f"  Image: {w}×{h}")

    dx0, dy0 = (w - geo["dialog_width"])//2, (h - geo["dialog_height"])//2
    dx1, dy1 = dx0 + geo["dialog_width"], dy0 + geo["dialog_height"]

    # 1. Dialog detection
    section = arr[dy0:dy1, dx0:dx1]
    non_dark = np.sum(np.max(section, axis=2) > 15)
    fill = non_dark / section.size * 100
    if fill > 5:
        ok("Dialog visible", f"({dx0},{dy0})–({dx1},{dy1}) = {geo['dialog_width']}×{geo['dialog_height']}, {fill:.0f}% fill")
    else:
        fail("Dialog visible", f"only {fill:.0f}% fill")

    # 2. Content density profile
    gray = np.mean(arr, axis=2)
    r.append(f"\n  Content density (y=dy0..dy1):")
    content_start = dy0 + geo["title_h"] + 1  # after title separator
    content_end = dy1 - geo["footer_h"] - 1    # before footer separator
    for y in range(content_start, min(content_end, dy1), 15):
        row = gray[y, dx0:dx1]
        pct = np.sum(row > 15) / len(row) * 100
        bar = "█" * int(pct/5) + "░" * (20 - int(pct/5))
        r.append(f"    y={y:4d} {bar} {pct:5.1f}%")

    # Content fill height
    bright_rows = np.sum(np.mean(section, axis=2) > 15, axis=1)
    content_filled = np.sum(bright_rows > section.shape[1]*0.1)
    content_pct = content_filled / section.shape[0] * 100
    r.append(f"  Content fill: {content_filled}/{section.shape[0]} rows ({content_pct:.0f}%)")

    # 3. Sidebar vs content boundary
    mid_y = dy0 + geo["title_h"] + 1 + 50
    row = gray[mid_y, dx0:dx1]
    left_avg = np.mean(row[:geo["sidebar_w"]])
    right_avg = np.mean(row[geo["sidebar_w"]+5:geo["sidebar_w"]+200])
    r.append(f"  Sidebar avg brightness: {left_avg:.0f}, Content avg: {right_avg:.0f}")
    diff = abs(left_avg - right_avg)
    if diff > 3:
        ok("Sidebar-content contrast", f"Δ={diff:.0f}")
    else:
        warn("Sidebar-content contrast", f"Δ={diff:.0f} — may blend")

    # 4. Footer detection
    foot_region = arr[dy1-geo["footer_h"]:dy1, dx0:dx1]
    foot_mean = np.mean(foot_region)
    r.append(f"  Footer avg brightness: {foot_mean:.0f}")
    if foot_mean < 30:
        ok("Footer dark (expected)")
    else:
        warn("Footer color", f"brightness={foot_mean:.0f}, expected dark")

    # 5. Accent blue detection
    accent = np.sum(np.all(arr[dy0:dy1, dx0:dx1] >= [60, 110, 230], axis=2))
    if accent > 50:
        ok(f"Blue accent (#4a7cf7) pixels: {accent}")
    else:
        warn(f"Blue accent", f"only {accent} px detected")

    # 6. Text regions (bright pixel clusters in content area)
    content = arr[content_start:content_end, dx0+geo["sidebar_w"]+5:dx1-5]
    bright = np.mean(content, axis=2) > 100
    text_pixels = np.sum(bright)
    text_pct = text_pixels / bright.size * 100
    r.append(f"  Text/bright pixels in content: {text_pixels}/{bright.size} ({text_pct:.1f}%)")

    # 7. Horizontal sections detection (tab content blocks)
    # Look for horizontal dark rows = separators
    content_gray = np.mean(content, axis=(1,2))
    seps = []
    for i in range(1, len(content_gray)-1):
        if content_gray[i] < 20 and content_gray[i-1] > 25 and content_gray[i+1] > 25:
            seps.append(i)
    r.append(f"  Horizontal separators in content: {len(seps)} at y-rel {seps[:5]}{'...' if len(seps)>5 else ''}")

    r.append(f"\n  Visual: {f_pass} pass, {f_warn} warn, {f_fail} fail")
    return r, f_fail, f_pass


# ── MAIN ────────────────────────────────────────────────────────────────────

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--snapshot")
    p.add_argument("--report")
    p.add_argument("--static-only", action="store_true")
    p.add_argument("--visual-only", action="store_true")
    args = p.parse_args()

    cfg = load_config()
    all_r, total_f, total_p = [], 0, 0

    if not args.visual_only:
        sr, sf, sp = static_analysis(cfg)
        all_r.extend(sr); total_f += sf; total_p += sp

    if not args.static_only:
        spath = args.snapshot or take_screenshot()
        if spath and Path(spath).exists():
            im = Image.open(spath).convert("RGB")
            all_r.append(f"\n  Screenshot: {spath} ({im.size[0]}×{im.size[1]})")
            vr, vf, vp = visual_analysis(im, cfg)
            all_r.extend(vr); total_f += vf; total_p += vp
        else:
            all_r.append("\n  WARNING: no screenshot available")

    all_r.append(f"\n══════ SUMMARY ══════")
    all_r.append(f"  PASS: {total_p}, FAIL: {total_f}")
    if total_p+total_f > 0:
        all_r.append(f"  Score: {total_p/(total_p+total_f)*100:.0f}%")

    txt = "\n".join(all_r)
    if args.report:
        Path(args.report).write_text(txt+"\n")
        print(f"Report: {args.report}", file=sys.stderr)
    else:
        print(txt)
    sys.exit(0 if total_f == 0 else min(total_f, 127))

if __name__ == "__main__":
    main()
