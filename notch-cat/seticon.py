"""실행 파일에 아이콘을 심는다: python seticon.py NotchCat.exe assets/cat.ico
리소스 컴파일러(rc·windres) 없이 윈도우의 UpdateResource 로 넣으므로 학교 PC·빌드 서버 어디서나 같다."""
import ctypes
import struct
import sys
from ctypes import wintypes

exe, ico = sys.argv[1], open(sys.argv[2], "rb").read()
_, kind, count = struct.unpack_from("<HHH", ico, 0)
assert kind == 1 and count > 0, "ico 파일이 아니다"

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
k32.BeginUpdateResourceW.restype = wintypes.HANDLE
k32.BeginUpdateResourceW.argtypes = [wintypes.LPCWSTR, wintypes.BOOL]
k32.UpdateResourceW.argtypes = [wintypes.HANDLE, wintypes.LPVOID, wintypes.LPVOID, wintypes.WORD, wintypes.LPVOID, wintypes.DWORD]
k32.EndUpdateResourceW.argtypes = [wintypes.HANDLE, wintypes.BOOL]

RT_ICON, RT_GROUP_ICON, LANG = 3, 14, 0x0409
h = k32.BeginUpdateResourceW(exe, False)
assert h, ctypes.WinError(ctypes.get_last_error())
group = struct.pack("<HHH", 0, 1, count)
for i in range(count):
    w, hgt, colors, _r, planes, bits, size, offset = struct.unpack_from("<BBBBHHII", ico, 6 + 16 * i)
    data = ico[offset:offset + size]
    assert k32.UpdateResourceW(h, RT_ICON, i + 1, LANG, data, len(data)), ctypes.WinError(ctypes.get_last_error())
    group += struct.pack("<BBBBHHIH", w, hgt, colors, 0, planes, bits, size, i + 1)
assert k32.UpdateResourceW(h, RT_GROUP_ICON, 1, LANG, group, len(group)), ctypes.WinError(ctypes.get_last_error())
assert k32.EndUpdateResourceW(h, False), ctypes.WinError(ctypes.get_last_error())
print("icon ok:", count, "sizes")
