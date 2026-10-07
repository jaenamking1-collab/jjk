# 바탕화면 고양이 — 윈도우 전용. 실행: run.bat (또는 pythonw desktop_cat.py)
# 고양이는 클릭이 통과하는 투명 창에 그려지고, 작업표시줄 트레이의 고양이 아이콘(우클릭)으로 설정·종료한다.
import os
import sys
import json
import time
import ctypes
import traceback
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
SETTINGS = os.path.join(HERE, 'settings.json')
LOG = os.path.join(HERE, 'desktop_cat.log')
TEST = os.environ.get('DESKTOP_CAT_TEST') == '1'   # 시험 모드: 캡처에 고양이가 찍히고, 2초마다 상태를 로그에
DEFAULTS = {'size': 1.0, 'color': 'art', 'fps': 40}
SIZES = [('작게', 0.75), ('보통', 1.0), ('크게', 1.35), ('아주 크게', 1.8)]
COLORS = [('그림(참고 그림 그대로)', 'art'), ('코드 그림: 하양', 'snow'), ('코드 그림: 치즈', 'cheese'),
          ('코드 그림: 회색', 'gray'), ('코드 그림: 검정', 'black')]
RUN_KEY = r'Software\Microsoft\Windows\CurrentVersion\Run'


def log(msg):
    try:
        with open(LOG, 'a', encoding='utf-8') as f:
            f.write(time.strftime('%Y-%m-%d %H:%M:%S ') + msg + '\n')
    except OSError:
        pass


def load_settings():
    s = dict(DEFAULTS)
    try:
        with open(SETTINGS, encoding='utf-8') as f:
            s.update(json.load(f))
    except (OSError, ValueError):
        pass
    return s


def save_settings(s):
    try:
        with open(SETTINGS, 'w', encoding='utf-8') as f:
            json.dump(s, f, ensure_ascii=False, indent=1)
    except OSError:
        pass


if sys.platform != 'win32':
    sys.exit('윈도우 전용입니다. 맥·리눅스에서는 preview.py 로 미리보기만 됩니다.')

import ctypes.wintypes as wt
import winreg

# 화면 배율(125%·150%)에서도 좌표가 실제 픽셀과 맞도록 — 다른 모듈보다 먼저
try:
    ctypes.windll.user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
except (AttributeError, OSError):
    try:
        ctypes.windll.shcore.SetProcessDpiAwareness(2)
    except (AttributeError, OSError):
        ctypes.windll.user32.SetProcessDPIAware()

from cat_render import CatRenderer, OX, OY
from sprite_render import SpriteRenderer
from cat_brain import CatBrain
from win_world import WorldScanner

user32 = ctypes.WinDLL('user32', use_last_error=True)
gdi32 = ctypes.WinDLL('gdi32')
kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
shell32 = ctypes.WinDLL('shell32')

LRESULT = ctypes.c_ssize_t
WNDPROC = ctypes.WINFUNCTYPE(LRESULT, wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM)


class WNDCLASSEXW(ctypes.Structure):
    _fields_ = [('cbSize', wt.UINT), ('style', wt.UINT), ('lpfnWndProc', WNDPROC), ('cbClsExtra', ctypes.c_int),
                ('cbWndExtra', ctypes.c_int), ('hInstance', wt.HINSTANCE), ('hIcon', wt.HICON),
                ('hCursor', wt.HANDLE), ('hbrBackground', wt.HBRUSH), ('lpszMenuName', wt.LPCWSTR),
                ('lpszClassName', wt.LPCWSTR), ('hIconSm', wt.HICON)]


class NOTIFYICONDATAW(ctypes.Structure):
    _fields_ = [('cbSize', wt.DWORD), ('hWnd', wt.HWND), ('uID', wt.UINT), ('uFlags', wt.UINT),
                ('uCallbackMessage', wt.UINT), ('hIcon', wt.HICON), ('szTip', wt.WCHAR * 128),
                ('dwState', wt.DWORD), ('dwStateMask', wt.DWORD), ('szInfo', wt.WCHAR * 256),
                ('uVersion', wt.UINT), ('szInfoTitle', wt.WCHAR * 64), ('dwInfoFlags', wt.DWORD),
                ('guidItem', ctypes.c_byte * 16), ('hBalloonIcon', wt.HICON)]


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [('biSize', wt.DWORD), ('biWidth', wt.LONG), ('biHeight', wt.LONG), ('biPlanes', wt.WORD),
                ('biBitCount', wt.WORD), ('biCompression', wt.DWORD), ('biSizeImage', wt.DWORD),
                ('biXPelsPerMeter', wt.LONG), ('biYPelsPerMeter', wt.LONG), ('biClrUsed', wt.DWORD),
                ('biClrImportant', wt.DWORD)]


class BLENDFUNCTION(ctypes.Structure):
    _fields_ = [('BlendOp', ctypes.c_ubyte), ('BlendFlags', ctypes.c_ubyte),
                ('SourceConstantAlpha', ctypes.c_ubyte), ('AlphaFormat', ctypes.c_ubyte)]


user32.DefWindowProcW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]
user32.DefWindowProcW.restype = LRESULT
user32.RegisterClassExW.argtypes = [ctypes.POINTER(WNDCLASSEXW)]
user32.CreateWindowExW.argtypes = [wt.DWORD, wt.LPCWSTR, wt.LPCWSTR, wt.DWORD, ctypes.c_int, ctypes.c_int,
                                   ctypes.c_int, ctypes.c_int, wt.HWND, wt.HMENU, wt.HINSTANCE, wt.LPVOID]
user32.CreateWindowExW.restype = wt.HWND
user32.UpdateLayeredWindow.argtypes = [wt.HWND, wt.HDC, ctypes.POINTER(wt.POINT), ctypes.POINTER(wt.SIZE),
                                       wt.HDC, ctypes.POINTER(wt.POINT), wt.COLORREF,
                                       ctypes.POINTER(BLENDFUNCTION), wt.DWORD]
user32.SetWindowPos.argtypes = [wt.HWND, wt.HWND, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, wt.UINT]
user32.PeekMessageW.argtypes = [ctypes.POINTER(wt.MSG), wt.HWND, wt.UINT, wt.UINT, wt.UINT]
user32.TrackPopupMenu.argtypes = [wt.HMENU, wt.UINT, ctypes.c_int, ctypes.c_int, ctypes.c_int, wt.HWND, ctypes.c_void_p]
user32.AppendMenuW.argtypes = [wt.HMENU, wt.UINT, ctypes.c_size_t, wt.LPCWSTR]
user32.CreatePopupMenu.restype = wt.HMENU
user32.LoadImageW.argtypes = [wt.HINSTANCE, wt.LPCWSTR, wt.UINT, ctypes.c_int, ctypes.c_int, wt.UINT]
user32.LoadImageW.restype = wt.HANDLE
user32.GetDC.restype = wt.HDC
gdi32.CreateCompatibleDC.argtypes = [wt.HDC]
gdi32.CreateCompatibleDC.restype = wt.HDC
gdi32.CreateDIBSection.argtypes = [wt.HDC, ctypes.POINTER(BITMAPINFOHEADER), wt.UINT,
                                   ctypes.POINTER(ctypes.c_void_p), wt.HANDLE, wt.DWORD]
gdi32.CreateDIBSection.restype = wt.HBITMAP
gdi32.SelectObject.argtypes = [wt.HDC, wt.HGDIOBJ]
gdi32.DeleteObject.argtypes = [wt.HGDIOBJ]
gdi32.DeleteDC.argtypes = [wt.HDC]
shell32.Shell_NotifyIconW.argtypes = [wt.DWORD, ctypes.POINTER(NOTIFYICONDATAW)]
kernel32.GetModuleHandleW.restype = wt.HMODULE
kernel32.CreateMutexW.restype = wt.HANDLE

WM_DESTROY, WM_CLOSE, WM_COMMAND, WM_APP = 0x0002, 0x0010, 0x0111, 0x8000
WM_TRAY = WM_APP + 1
WM_LBUTTONUP, WM_RBUTTONUP = 0x0202, 0x0205


class Overlay:
    """고양이 그림 한 장 크기의 투명 창. 매 프레임 그림과 위치를 한 번에 바꾼다(UpdateLayeredWindow)."""

    def __init__(self, hinst, wndproc, w, h):
        self.w, self.h = w, h
        ex = 0x00080000 | 0x00000020 | 0x00000008 | 0x00000080 | 0x08000000  # LAYERED|TRANSPARENT|TOPMOST|TOOLWINDOW|NOACTIVATE
        self.hwnd = user32.CreateWindowExW(ex, 'DesktopCatWnd', 'DesktopCat', 0x80000000, 0, 0, w, h,
                                           None, None, hinst, None)
        try:                                       # 화면 캡처에 고양이 자신이 찍히지 않게(윈10 2004+)
            if not TEST:
                user32.SetWindowDisplayAffinity(self.hwnd, 0x11)
        except (AttributeError, OSError):
            pass
        self.screen = user32.GetDC(None)
        self.mdc = gdi32.CreateCompatibleDC(self.screen)
        self.bmp = None
        self._alloc(w, h)
        user32.ShowWindow(self.hwnd, 4)            # SW_SHOWNOACTIVATE
        self.hidden = False

    def _alloc(self, w, h):
        if self.bmp:
            gdi32.DeleteObject(self.bmp)
        self.w, self.h = w, h
        bmi = BITMAPINFOHEADER(40, w, -h, 1, 32, 0, 0, 0, 0, 0, 0)
        self.bits = ctypes.c_void_p()
        self.bmp = gdi32.CreateDIBSection(self.mdc, ctypes.byref(bmi), 0, ctypes.byref(self.bits), None, 0)
        gdi32.SelectObject(self.mdc, self.bmp)

    def show(self, img, x, y):
        if img.size != (self.w, self.h):
            self._alloc(*img.size)
        a = np.asarray(img, dtype=np.uint16)
        al = a[..., 3]
        bgra = np.empty((self.h, self.w, 4), np.uint8)       # 알파 미리곱하기 + BGRA 순서
        for i, c in enumerate((2, 1, 0)):
            bgra[..., i] = (a[..., c] * al + 127) // 255
        bgra[..., 3] = al
        ctypes.memmove(self.bits, bgra.ctypes.data, bgra.nbytes)
        pt, sz, src = wt.POINT(int(x), int(y)), wt.SIZE(self.w, self.h), wt.POINT(0, 0)
        blend = BLENDFUNCTION(0, 0, 255, 1)
        user32.UpdateLayeredWindow(self.hwnd, self.screen, ctypes.byref(pt), ctypes.byref(sz), self.mdc,
                                   ctypes.byref(src), 0, ctypes.byref(blend), 2)

    def topmost(self):
        user32.SetWindowPos(self.hwnd, wt.HWND(-1), 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0010)

    def set_hidden(self, hidden):
        self.hidden = hidden
        user32.ShowWindow(self.hwnd, 0 if hidden else 4)


def autostart_get():
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, RUN_KEY) as k:
            winreg.QueryValueEx(k, 'DesktopCat')
            return True
    except OSError:
        return False


def autostart_set(on):
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, RUN_KEY, 0, winreg.KEY_SET_VALUE) as k:
        if on:
            exe = sys.executable
            w = os.path.join(os.path.dirname(exe), 'pythonw.exe')
            winreg.SetValueEx(k, 'DesktopCat', 0, winreg.REG_SZ,
                              '"%s" "%s"' % (w if os.path.exists(w) else exe, os.path.abspath(__file__)))
        else:
            try:
                winreg.DeleteValue(k, 'DesktopCat')
            except OSError:
                pass


class App:
    def __init__(self):
        self.s = load_settings()
        self.running = True
        self.hinst = kernel32.GetModuleHandleW(None)
        self._proc = WNDPROC(self.wndproc)         # 참조를 붙잡아 둬야 콜백이 안 사라진다
        for name in ('DesktopCatWnd', 'DesktopCatTray'):
            wc = WNDCLASSEXW()
            wc.cbSize = ctypes.sizeof(wc)
            wc.lpfnWndProc = self._proc
            wc.hInstance = self.hinst
            wc.lpszClassName = name
            user32.RegisterClassExW(ctypes.byref(wc))
        self.tray_hwnd = user32.CreateWindowExW(0, 'DesktopCatTray', 'DesktopCatTray', 0, 0, 0, 0, 0,
                                                None, None, self.hinst, None)
        self.dpi = user32.GetDpiForSystem() if hasattr(user32, 'GetDpiForSystem') else 96
        self.build()
        self.overlay = Overlay(self.hinst, self._proc, self.rend.W, self.rend.H)
        self.scanner = WorldScanner([self.overlay.hwnd, self.tray_hwnd])
        self.scanner.start()
        self.add_tray()

    def build(self):
        self.scale = 1.3 * self.s['size'] * self.dpi / 96
        if self.s['color'] == 'art':
            self.rend = SpriteRenderer(self.scale)
            self.ox, self.oy = self.rend.ox, self.rend.oy
        else:
            self.rend = CatRenderer(self.scale, self.s['color'])
            self.ox, self.oy = OX * self.scale, OY * self.scale
        old = getattr(self, 'brain', None)
        self.brain = CatBrain(self.scale)
        if old is not None and old.spawned:        # 크기·색만 바꿨으면 그 자리에서 이어서
            self.brain.x, self.brain.y, self.brain.spawned = old.x, old.y, True
            self.brain.mode, self.brain.vy = 'fall', 0.0

    # ── 트레이 ─────────────────────────────────────────────
    def add_tray(self):
        ico = os.path.join(HERE, 'cat.ico')
        try:
            from cat_render import action_pose
            if self.s['color'] == 'art':               # 트레이 아이콘: 웃는 고양이 그림
                from PIL import Image
                im = Image.open(os.path.join(HERE, 'sprites', 'happy.png')).convert('RGBA')
                side = max(im.size)
                sq = Image.new('RGBA', (side, side), (0, 0, 0, 0))
                sq.alpha_composite(im, ((side - im.width) // 2, side - im.height))
                sq.save(ico, sizes=[(16, 16), (32, 32), (48, 48)])
            else:
                im = CatRenderer(1.0, self.s['color']).render(action_pose('sit', 0, {}), 1, 0)
                im.crop((OX - 24, OY - 48, OX + 24, OY)).save(ico, sizes=[(16, 16), (32, 32), (48, 48)])
            hicon = user32.LoadImageW(None, ico, 1, 0, 0, 0x10 | 0x40)
        except Exception:
            hicon = None
        if not hicon:
            hicon = user32.LoadIconW(None, ctypes.c_wchar_p(32512))
        nid = self.nid = NOTIFYICONDATAW()
        nid.cbSize = ctypes.sizeof(nid)
        nid.hWnd, nid.uID, nid.uFlags = self.tray_hwnd, 1, 0x1 | 0x2 | 0x4
        nid.uCallbackMessage, nid.hIcon, nid.szTip = WM_TRAY, hicon, '바탕화면 고양이 (우클릭: 메뉴)'
        shell32.Shell_NotifyIconW(0, ctypes.byref(nid))

    def menu(self):
        m = user32.CreatePopupMenu()
        ck = lambda on: 0x8 if on else 0          # MF_CHECKED
        user32.AppendMenuW(m, 0, 1, '고양이 부르기 (마우스 있는 모니터로)')
        user32.AppendMenuW(m, ck(self.overlay.hidden), 2, '잠깐 숨기기')
        user32.AppendMenuW(m, 0x800, 0, None)
        for i, (label, v) in enumerate(SIZES):
            user32.AppendMenuW(m, ck(abs(self.s['size'] - v) < 1e-6), 10 + i, '크기: ' + label)
        user32.AppendMenuW(m, 0x800, 0, None)
        for i, (label, v) in enumerate(COLORS):
            user32.AppendMenuW(m, ck(self.s['color'] == v), 20 + i, '색: ' + label)
        user32.AppendMenuW(m, 0x800, 0, None)
        user32.AppendMenuW(m, ck(autostart_get()), 3, '윈도우 켤 때 자동 실행')
        user32.AppendMenuW(m, 0, 9, '종료')
        pt = wt.POINT()
        user32.GetCursorPos(ctypes.byref(pt))
        user32.SetForegroundWindow(self.tray_hwnd)
        cmd = user32.TrackPopupMenu(m, 0x100 | 0x2, pt.x, pt.y, 0, self.tray_hwnd, None)
        user32.DestroyMenu(m)
        self.command(cmd, pt)

    def command(self, cmd, pt):
        if cmd == 1:
            self.overlay.set_hidden(False)
            snap = self.scanner.snapshot()
            if snap:
                self.brain.call_to(pt.x, snap['platforms'])
        elif cmd == 2:
            self.overlay.set_hidden(not self.overlay.hidden)
        elif cmd == 3:
            autostart_set(not autostart_get())
        elif cmd == 9:
            self.running = False
        elif 10 <= cmd < 10 + len(SIZES):
            self.s['size'] = SIZES[cmd - 10][1]
            save_settings(self.s)
            self.build()
        elif 20 <= cmd < 20 + len(COLORS):
            self.s['color'] = COLORS[cmd - 20][1]
            save_settings(self.s)
            self.build()

    def wndproc(self, hwnd, msg, wp, lp):
        if hwnd == self.tray_hwnd and msg == WM_TRAY and lp in (WM_LBUTTONUP, WM_RBUTTONUP):
            self.menu()
            return 0
        if msg in (WM_CLOSE, WM_DESTROY) and hwnd == self.tray_hwnd:
            self.running = False
            return 0
        return user32.DefWindowProcW(hwnd, msg, wp, lp)

    # ── 메인 루프 ─────────────────────────────────────────
    def run(self):
        msg = wt.MSG()
        frame = 1.0 / max(10, int(self.s.get('fps', 40)))
        last = time.perf_counter()
        t_top = t_log = 0.0
        while self.running:
            while user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1):
                if msg.message == 0x0012:          # WM_QUIT
                    self.running = False
                user32.TranslateMessage(ctypes.byref(msg))
                user32.DispatchMessageW(ctypes.byref(msg))
            now = time.perf_counter()
            dt, last = now - last, now
            snap = self.scanner.snapshot()
            if snap and snap['monitors'] and not self.overlay.hidden:
                pt = wt.POINT()
                user32.GetCursorPos(ctypes.byref(pt))
                world = dict(snap, cursor=(pt.x, pt.y))
                x, y, facing, pose = self.brain.update(dt, world)
                if isinstance(self.rend, SpriteRenderer):
                    img = self.rend.render_action(self.brain.action, self.brain.at, facing, self.brain.time, self.brain)
                else:
                    img = self.rend.render(pose, facing, self.brain.time)
                left, top = x - self.ox, y - self.oy
                self.scanner.cat = (x, y, (int(left), int(top), int(left) + img.width, int(top) + img.height))
                self.overlay.show(img, left, top)
                if now > t_top:
                    self.overlay.topmost()
                    t_top = now + 2.0
            if TEST and snap and now > t_log:
                kinds = {}
                for p in snap['platforms']:
                    kinds[p.kind] = kinds.get(p.kind, 0) + 1
                log('test mode=%s action=%s x=%d y=%d plats=%s' % (self.brain.mode, self.brain.action,
                                                                   self.brain.x, self.brain.y, kinds))
                t_log = now + 2.0
            if self.scanner.error:
                log('scanner: ' + self.scanner.error)
                self.scanner.error = None
            rest = frame - (time.perf_counter() - now)
            if rest > 0:
                time.sleep(rest)
        self.scanner.running = False
        shell32.Shell_NotifyIconW(2, ctypes.byref(self.nid))


def main():
    kernel32.CreateMutexW(None, False, 'DesktopCat_single_instance')
    if ctypes.get_last_error() == 183:                                      # 이미 실행 중
        return
    try:
        ctypes.windll.winmm.timeBeginPeriod(1)    # sleep 정밀도 1ms (기본 15ms 라 끊겨 보인다)
    except OSError:
        pass
    try:
        App().run()
    except Exception:
        log(traceback.format_exc())
        raise


if __name__ == '__main__':
    main()
