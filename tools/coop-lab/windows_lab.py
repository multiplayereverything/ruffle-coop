"""Ruffle Together windows lab: is a session smooth on screen?

Opens two real session windows on this PC (host and friend, scripted keys), lets them play, and
checks each window's "Ruffle Together timing" log lines against pass limits. Runs two modes:

  quiet     nobody touches the windows
  pressure  mouse-move messages are posted to the two Ruffle windows only (125 per second), the way
            a player's mouse and keys make the window repaint; the real mouse and keyboard are never
            touched

  python tools/coop-lab/windows_lab.py --swf <BadIceCream_coop.swf> [--exe <ruffle_desktop.exe>]
      [--mode quiet|pressure|both] [--seconds 40] [--delay N]

Exit code 0 only if every window in every mode passes.
"""
import argparse
import ctypes
import glob
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
from ctypes import wintypes

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
RUFFLE = os.path.join(ROOT, "target", "release", "ruffle_desktop.exe")
SCRIPT = os.path.join(ROOT, "desktop", "tests", "coop", "bic-two-players.txt")
RESERVED = {6420, 7777, 7778, 7779, 27015, 27016, 27020, 27021}

# one line every 5 s; the first two cover start-up and are not judged
WARMUP_LINES = 2
LIMITS = {
    "fps": 29.5,           # game frames per second, at least
    "shown": (28.0, 32.5),  # pictures on screen per second (one per game frame)
    "late_total": 3,       # frames > 1.25 frame times after the previous one, whole run (a level load costs ~2)
    "ticks": 5000,         # event-loop passes per 5 s, at most (more means a busy loop)
}

TIMING = re.compile(
    r"timing: (?P<fps>[\d.]+) fps \(game \d+\), (?P<shown>[\d.]+) shown per second, (?P<ticks>\d+) ticks, "
    r"longest tick gap (?P<gap>\d+) ms, frame cost avg (?P<cost>[\d.]+) / max (?P<cost_max>\d+) ms, "
    r"waited for input (?P<wait>\d+) ms of \d+ \(longest (?P<wait_max>\d+) ms\), (?P<late>\d+) late frames, "
    r"(?P<doubled>\d+) doubled ticks, input delay avg (?P<delay>\d+) / max (?P<delay_max>\d+) ms")
PRINT = re.compile(r"frame (\d+), fingerprint ([0-9a-f]+)")


def free_port():
    while True:
        with socket.socket() as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
        if port not in RESERVED:
            return port


def windows_of(pids):
    """Top-level visible windows owned by these processes."""
    user32 = ctypes.windll.user32
    found = []

    @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    def each(hwnd, _):
        pid = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
        if pid.value in pids and user32.IsWindowVisible(hwnd):
            found.append(hwnd)
        return True

    user32.EnumWindows(each, 0)
    return found


def pressure(pids, stop):
    """Posts WM_MOUSEMOVE to the session windows only, 125 times a second."""
    user32 = ctypes.windll.user32
    wm_mousemove = 0x0200
    step = 0
    while not stop.is_set():
        for hwnd in windows_of(pids):
            x, y = 100 + step % 300, 100 + (step * 7) % 300
            user32.PostMessageW(hwnd, wm_mousemove, 0, (y << 16) | x)
        step += 1
        time.sleep(0.008)


def judge(name, lines, prints):
    rows = [m.groupdict() for m in map(TIMING.search, lines) if m][WARMUP_LINES:]
    if not rows:
        return False, f"{name}: no timing lines (wrong build?)"
    fps = min(float(r["fps"]) for r in rows)
    shown = [float(r["shown"]) for r in rows]
    late = sum(int(r["late"]) for r in rows)
    doubled = sum(int(r["doubled"]) for r in rows)
    ticks = max(int(r["ticks"]) for r in rows)
    delay = sum(int(r["delay"]) for r in rows) / len(rows)
    delay_max = max(int(r["delay_max"]) for r in rows)
    fails = []
    if fps < LIMITS["fps"]:
        fails.append(f"fps {fps}")
    lo, hi = LIMITS["shown"]
    if min(shown) < lo or max(shown) > hi:
        fails.append(f"shown {min(shown)}-{max(shown)}/s")
    if late > LIMITS["late_total"]:
        fails.append(f"{late} late frames")
    if ticks > LIMITS["ticks"]:
        fails.append(f"busy loop ({ticks} ticks per 5 s)")
    text = (f"{name}: fps min {fps}, shown {min(shown)}-{max(shown)}/s, late {late}, doubled {doubled}, "
            f"ticks max {ticks}, input delay avg {delay:.0f} / max {delay_max} ms, sync checks {len(prints)}")
    return not fails, text + ("" if not fails else "  FAIL: " + ", ".join(fails))


def run(args, mode):
    work = tempfile.mkdtemp(prefix="coop-windows-lab-")
    port = free_port()
    common = ["--coop-script", SCRIPT]
    delay = ["--coop-delay", str(args.delay)] if args.delay is not None else []
    host = subprocess.Popen([args.exe, "--cache-directory", os.path.join(work, "host"), "--coop-host", str(port),
                             "--coop-name", "Host"] + delay + common + [args.swf],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(2)
    friend = subprocess.Popen([args.exe, "--cache-directory", os.path.join(work, "friend"), "--coop-join",
                               "127.0.0.1:%d" % port, "--coop-name", "Friend"] + common + [args.swf],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    stop = threading.Event()
    if mode == "pressure":
        threading.Thread(target=pressure, args=({host.pid, friend.pid}, stop), daemon=True).start()
    time.sleep(args.seconds)
    stop.set()
    for proc in (host, friend):
        proc.kill()
        proc.wait()

    ok, prints = True, {}
    print(f"== {mode}")
    for name in ("host", "friend"):
        lines = []
        for path in glob.glob(os.path.join(work, name, "log", "*.log")):
            lines += open(path, encoding="utf-8", errors="replace").read().splitlines()
        prints[name] = dict(m.groups() for m in map(PRINT.search, lines) if m)
        if any("OUT OF SYNC" in line for line in lines):
            ok = False
            print(f"{name}: OUT OF SYNC")
        passed, text = judge(name, lines, prints[name])
        ok &= passed
        print(text)
    if prints["host"] != prints["friend"]:
        ok = False
        print(f"FAIL: fingerprints differ: host {prints['host']} friend {prints['friend']}")
    shutil.rmtree(work, ignore_errors=True)
    print("PASS" if ok else "FAIL")
    return ok


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--swf", required=True)
    parser.add_argument("--exe", default=RUFFLE)
    parser.add_argument("--mode", choices=["quiet", "pressure", "both"], default="both")
    parser.add_argument("--seconds", type=int, default=40)
    parser.add_argument("--delay", type=int, help="input delay, frames (default: the host picks it from the ping)")
    args = parser.parse_args()
    modes = ["quiet", "pressure"] if args.mode == "both" else [args.mode]
    results = [run(args, mode) for mode in modes]
    sys.exit(0 if all(results) else 1)


if __name__ == "__main__":
    main()
