# 윈도우 바탕화면에서 고양이가 밟을 발판을 읽는다: 모니터 바닥(작업표시줄 위), 창 윗변, 바탕화면 아이콘,
# 그리고 화면 캡처로 찾은 그림·글자의 윗선. 뒤쪽 스레드에서 돌고 snapshot() 으로 꺼내 쓴다.
import sys
import threading
import time
import ctypes
import numpy as np
from cat_brain import Platform, ground_platforms

if sys.platform == 'win32':
    import ctypes.wintypes as wt
    user32 = ctypes.WinDLL('user32', use_last_error=True)
    gdi32 = ctypes.WinDLL('gdi32')
    kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
    dwmapi = ctypes.WinDLL('dwmapi')

    class MONITORINFO(ctypes.Structure):
        _fields_ = [('cbSize', wt.DWORD), ('rcMonitor', wt.RECT), ('rcWork', wt.RECT), ('dwFlags', wt.DWORD)]

    class BITMAPINFOHEADER(ctypes.Structure):
        _fields_ = [('biSize', wt.DWORD), ('biWidth', wt.LONG), ('biHeight', wt.LONG), ('biPlanes', wt.WORD),
                    ('biBitCount', wt.WORD), ('biCompression', wt.DWORD), ('biSizeImage', wt.DWORD),
                    ('biXPelsPerMeter', wt.LONG), ('biYPelsPerMeter', wt.LONG), ('biClrUsed', wt.DWORD),
                    ('biClrImportant', wt.DWORD)]

    MonitorEnumProc = ctypes.WINFUNCTYPE(wt.BOOL, wt.HMONITOR, wt.HDC, ctypes.POINTER(wt.RECT), wt.LPARAM)
    EnumWindowsProc = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)

    user32.EnumDisplayMonitors.argtypes = [wt.HDC, ctypes.c_void_p, MonitorEnumProc, wt.LPARAM]
    user32.GetMonitorInfoW.argtypes = [wt.HMONITOR, ctypes.POINTER(MONITORINFO)]
    user32.EnumWindows.argtypes = [EnumWindowsProc, wt.LPARAM]
    user32.GetClassNameW.argtypes = [wt.HWND, wt.LPWSTR, ctypes.c_int]
    user32.GetWindowLongW.argtypes = [wt.HWND, ctypes.c_int]
    user32.FindWindowW.argtypes = [wt.LPCWSTR, wt.LPCWSTR]
    user32.FindWindowW.restype = wt.HWND
    user32.FindWindowExW.argtypes = [wt.HWND, wt.HWND, wt.LPCWSTR, wt.LPCWSTR]
    user32.FindWindowExW.restype = wt.HWND
    user32.SendMessageTimeoutW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM, wt.UINT, wt.UINT,
                                           ctypes.POINTER(ctypes.c_size_t)]
    user32.SendMessageTimeoutW.restype = wt.LPARAM
    user32.MapWindowPoints.argtypes = [wt.HWND, wt.HWND, ctypes.POINTER(wt.POINT), wt.UINT]
    user32.GetDC.restype = wt.HDC
    user32.ReleaseDC.argtypes = [wt.HWND, wt.HDC]
    dwmapi.DwmGetWindowAttribute.argtypes = [wt.HWND, wt.DWORD, ctypes.c_void_p, wt.DWORD]
    kernel32.OpenProcess.restype = wt.HANDLE
    kernel32.VirtualAllocEx.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_size_t, wt.DWORD, wt.DWORD]
    kernel32.VirtualAllocEx.restype = ctypes.c_void_p
    kernel32.VirtualFreeEx.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_size_t, wt.DWORD]
    kernel32.WriteProcessMemory.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t,
                                            ctypes.POINTER(ctypes.c_size_t)]
    kernel32.ReadProcessMemory.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t,
                                           ctypes.POINTER(ctypes.c_size_t)]
    kernel32.CloseHandle.argtypes = [wt.HANDLE]
    gdi32.CreateCompatibleDC.argtypes = [wt.HDC]
    gdi32.CreateCompatibleDC.restype = wt.HDC
    gdi32.CreateCompatibleBitmap.argtypes = [wt.HDC, ctypes.c_int, ctypes.c_int]
    gdi32.CreateCompatibleBitmap.restype = wt.HBITMAP
    gdi32.SelectObject.argtypes = [wt.HDC, wt.HGDIOBJ]
    gdi32.SelectObject.restype = wt.HGDIOBJ
    gdi32.BitBlt.argtypes = [wt.HDC, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, wt.HDC,
                             ctypes.c_int, ctypes.c_int, wt.DWORD]
    gdi32.GetDIBits.argtypes = [wt.HDC, wt.HBITMAP, wt.UINT, wt.UINT, ctypes.c_void_p,
                                ctypes.POINTER(BITMAPINFOHEADER), wt.UINT]
    gdi32.DeleteObject.argtypes = [wt.HGDIOBJ]
    gdi32.DeleteDC.argtypes = [wt.HDC]

# 창 윗변을 발판으로 쓰지 않을 창 종류(바탕화면·작업표시줄·시스템 오버레이)
SKIP_CLASS = {'Progman', 'WorkerW', 'Shell_TrayWnd', 'Shell_SecondaryTrayWnd', 'Windows.UI.Core.CoreWindow',
              'NotifyIconOverflowWindow', 'TopLevelWindowForOverflowXamlIsland', 'XamlExplorerHostIslandWindow',
              'DesktopCatWnd', 'DesktopCatTray'}


# ── 모니터 ─────────────────────────────────────────────────
def monitors():
    res = []

    def cb(hm, hdc, lprc, lp):
        mi = MONITORINFO()
        mi.cbSize = ctypes.sizeof(mi)
        user32.GetMonitorInfoW(hm, ctypes.byref(mi))
        r, w = mi.rcMonitor, mi.rcWork
        res.append(dict(rect=(r.left, r.top, r.right, r.bottom), work=(w.left, w.top, w.right, w.bottom),
                        primary=bool(mi.dwFlags & 1)))
        return True

    user32.EnumDisplayMonitors(None, None, MonitorEnumProc(cb), 0)
    res.sort(key=lambda m: not m['primary'])
    return res


# ── 창 ────────────────────────────────────────────────────
def windows(own=()):
    """보이는 일반 창을 위(앞)에 있는 것부터 순서대로."""
    out = []
    buf = ctypes.create_unicode_buffer(128)

    def cb(h, lp):
        if h in own or not user32.IsWindowVisible(h) or user32.IsIconic(h):
            return True
        if user32.GetWindowLongW(h, -20) & 0x80:          # WS_EX_TOOLWINDOW
            return True
        if user32.GetWindowTextLengthW(h) == 0:
            return True
        user32.GetClassNameW(h, buf, 128)
        if buf.value in SKIP_CLASS:
            return True
        cloaked = ctypes.c_int(0)                          # 다른 가상 데스크톱·숨은 UWP 창
        dwmapi.DwmGetWindowAttribute(h, 14, ctypes.byref(cloaked), 4)
        if cloaked.value:
            return True
        r = wt.RECT()
        if dwmapi.DwmGetWindowAttribute(h, 9, ctypes.byref(r), ctypes.sizeof(r)) != 0:
            user32.GetWindowRect(h, ctypes.byref(r))
        if r.right - r.left < 120 or r.bottom - r.top < 60:
            return True
        out.append((h, (r.left, r.top, r.right, r.bottom)))
        return True

    user32.EnumWindows(EnumWindowsProc(cb), 0)
    return out


def _subtract(segs, a, b):
    out = []
    for x1, x2 in segs:
        if b <= x1 or a >= x2:
            out.append((x1, x2))
        else:
            if a > x1:
                out.append((x1, a))
            if b < x2:
                out.append((b, x2))
    return out


def window_platforms(wins):
    """창 윗변 — 그보다 앞에 있는 창에 가려진 구간은 뺀다."""
    plats, refs = [], {}
    for i, (h, (l, t, r, b)) in enumerate(wins):
        segs = [(l, r)]
        for _, (l2, t2, r2, b2) in wins[:i]:
            if t2 <= t - 1 <= b2:
                segs = _subtract(segs, l2, r2)
        for x1, x2 in segs:
            if x2 - x1 >= 20:
                plats.append(Platform(x1, x2, t, 'window', ('win', h)))
        refs[('win', h)] = (l, t)
    return plats, refs


# ── 바탕화면 아이콘 (탐색기의 목록 컨트롤에서 위치를 읽는다) ──────────
class IconReader:
    def __init__(self):
        self.lv = self.hproc = self.mem = None

    @staticmethod
    def _listview():
        dv = user32.FindWindowExW(user32.FindWindowW('Progman', None), None, 'SHELLDLL_DefView', None)
        if not dv:
            found = []

            def cb(h, lp):
                d = user32.FindWindowExW(h, None, 'SHELLDLL_DefView', None)
                if d:
                    found.append(d)
                    return False
                return True

            user32.EnumWindows(EnumWindowsProc(cb), 0)
            dv = found[0] if found else None
        return user32.FindWindowExW(dv, None, 'SysListView32', None) if dv else None

    def _close(self):
        if self.hproc:
            if self.mem:
                kernel32.VirtualFreeEx(self.hproc, self.mem, 0, 0x8000)
            kernel32.CloseHandle(self.hproc)
        self.lv = self.hproc = self.mem = None

    def _open(self):
        self._close()
        lv = self._listview()
        if not lv:
            return False
        pid = wt.DWORD()
        user32.GetWindowThreadProcessId(lv, ctypes.byref(pid))
        hp = kernel32.OpenProcess(0x0008 | 0x0010 | 0x0020 | 0x0400, False, pid.value)
        if not hp:
            return False
        mem = kernel32.VirtualAllocEx(hp, None, 4096, 0x3000, 0x04)
        if not mem:
            kernel32.CloseHandle(hp)
            return False
        self.lv, self.hproc, self.mem = lv, hp, mem
        return True

    def _send(self, msg, wp, lp):
        res = ctypes.c_size_t()
        ok = user32.SendMessageTimeoutW(self.lv, msg, wp, lp, 0x0002, 300, ctypes.byref(res))
        return res.value if ok else None

    def read(self):
        if not self.lv or not user32.IsWindow(self.lv):
            if not self._open():
                return []
        if not user32.IsWindowVisible(self.lv):           # '바탕화면 아이콘 표시'가 꺼져 있음
            return []
        n = self._send(0x1004, 0, 0)                      # LVM_GETITEMCOUNT
        if n is None:
            self._close()
            return []
        out = []
        for i in range(min(n, 400)):
            rc = wt.RECT(1, 0, 0, 0)                      # LVIR_ICON: 그림 부분만
            kernel32.WriteProcessMemory(self.hproc, self.mem, ctypes.byref(rc), 16, None)
            if not self._send(0x100E, i, self.mem):       # LVM_GETITEMRECT
                continue
            kernel32.ReadProcessMemory(self.hproc, self.mem, ctypes.byref(rc), 16, None)
            pts = (wt.POINT * 2)(wt.POINT(rc.left, rc.top), wt.POINT(rc.right, rc.bottom))
            user32.MapWindowPoints(self.lv, None, pts, 2)
            out.append((pts[0].x, pts[0].y, pts[1].x, pts[1].y))
        return out


def icon_platforms(rects, wins):
    plats = []
    for i, (l, t, r, b) in enumerate(rects):
        if r - l < 8:
            continue
        cx = (l + r) // 2
        if any(wl <= cx <= wr and wt_ <= t - 2 <= wb for _, (wl, wt_, wr, wb) in wins):
            continue                                      # 창에 가려진 아이콘
        pad = (r - l) // 10
        plats.append(Platform(l + pad, r - pad, t, 'icon', ('icon', i)))
    return plats


# ── 화면 속 그림·글자의 윗선 ───────────────────────────────────
def grab_gray(x, y, w, h):
    hdc = user32.GetDC(None)
    mdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    old = gdi32.SelectObject(mdc, bmp)
    gdi32.BitBlt(mdc, 0, 0, w, h, hdc, x, y, 0x00CC0020)  # SRCCOPY
    bmi = BITMAPINFOHEADER(40, w, -h, 1, 32, 0, 0, 0, 0, 0, 0)
    buf = (ctypes.c_ubyte * (w * h * 4))()
    gdi32.GetDIBits(mdc, bmp, 0, h, buf, ctypes.byref(bmi), 0)
    gdi32.SelectObject(mdc, old)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mdc)
    user32.ReleaseDC(None, hdc)
    a = np.frombuffer(buf, np.uint8).reshape(h, w, 4).astype(np.int16)
    return (a[..., 2] * 77 + a[..., 1] * 150 + a[..., 0] * 29) >> 8


def find_edges(g, ox, oy, min_len=24, gap=3, mask=None, limit=500):
    """밝기가 위아래로 확 바뀌고 그 위쪽은 잔잔한 가로선 = 물체의 윗선. 화면 좌표의 (x1, x2, y) 목록."""
    g = g.astype(np.int16)
    d = np.abs(g[1:] - g[:-1])                            # d[i]: i행과 i+1행 사이
    strong, calm = d > 40, d < 10
    c = strong[3:] & calm[2:-1] & calm[1:-2] & calm[:-3]  # c[j] → 윗선은 j+4 행
    if mask is not None:                                  # 고양이 자신은 빼고
        mx1, my1, mx2, my2 = mask
        c[max(0, my1 - oy - 4):max(0, my2 - oy - 4), max(0, mx1 - ox):max(0, mx2 - ox)] = False
    dil = c.copy()                                        # 글자 사이 작은 틈은 메운다
    for s in range(1, gap + 1):
        dil[:, s:] |= c[:, :-s]
        dil[:, :-s] |= c[:, s:]
    clo = dil.copy()
    for s in range(1, gap + 1):
        clo[:, s:] &= dil[:, :-s]
        clo[:, :-s] &= dil[:, s:]
    segs = []
    for r in np.nonzero(clo.sum(1) >= min_len)[0]:
        dif = np.diff(np.concatenate(([0], clo[r].astype(np.int8), [0])))
        for a, b in zip(np.nonzero(dif == 1)[0], np.nonzero(dif == -1)[0]):
            if b - a >= min_len:
                segs.append((int(ox + a), int(ox + b), int(oy + r + 4)))
    segs.sort(key=lambda s: s[2])
    kept = []                                             # 1~3px 아래 겹치는 같은 선은 하나로
    for s in segs:
        if not any(k[2] < s[2] <= k[2] + 3 and min(k[1], s[1]) - max(k[0], s[0]) > 0.5 * (s[1] - s[0])
                   for k in kept[-40:]):
            kept.append(s)
    kept.sort(key=lambda s: s[0] - s[1])
    return kept[:limit]


# ── 뒤쪽 스레드 ────────────────────────────────────────────
class WorldScanner(threading.Thread):
    def __init__(self, own_hwnds, edge_radius=(800, 520)):
        super().__init__(daemon=True)
        self.own = set(own_hwnds)
        self.cat = None                                   # 메인 루프가 (x, y, 고양이 사각형) 을 넣어 준다
        self.edge_radius = edge_radius
        self.lock = threading.Lock()
        self._snap = None
        self.running = True
        self.error = None

    def snapshot(self):
        with self.lock:
            return self._snap

    def run(self):
        icons = IconReader()
        mons, wins, icon_rects, edges = [], [], [], []
        t_mon = t_win = t_icon = t_edge = 0.0
        while self.running:
            now = time.monotonic()
            try:
                if now >= t_mon:
                    mons, t_mon = monitors(), now + 3.0
                if now >= t_win:
                    wins, t_win = windows(self.own), now + 0.2
                if now >= t_icon:
                    icon_rects, t_icon = icons.read(), now + 2.0
                if now >= t_edge and self.cat:
                    edges, t_edge = self._edges(mons), now + 0.6
                wp, refs = window_platforms(wins)
                plats = ground_platforms(mons) + wp + icon_platforms(icon_rects, wins) + \
                    [Platform(a, b, y, 'edge') for a, b, y in edges]
                with self.lock:
                    self._snap = dict(monitors=mons, platforms=plats, refs=refs)
            except Exception as e:                        # 한 번 실패해도 계속 돈다
                self.error = repr(e)
            time.sleep(0.05)

    def _edges(self, mons):
        x, y, rect = self.cat
        vx1 = min(m['rect'][0] for m in mons)
        vy1 = min(m['rect'][1] for m in mons)
        vx2 = max(m['rect'][2] for m in mons)
        vy2 = max(m['rect'][3] for m in mons)
        rx, ry = self.edge_radius
        x1, y1 = max(vx1, int(x - rx)), max(vy1, int(y - ry))
        x2, y2 = min(vx2, int(x + rx)), min(vy2, int(y + ry * 0.6))
        if x2 - x1 < 40 or y2 - y1 < 40:
            return []
        g = grab_gray(x1, y1, x2 - x1, y2 - y1)
        return find_edges(g, x1, y1, mask=rect)
