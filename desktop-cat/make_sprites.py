# 그림 시트를 동작별 투명 PNG 로 오린다. 그림을 새로 받았을 때만 돌린다.
#   pip install "rembg[cpu]" scipy
#   python make_sprites.py s1=첫시트.png s3=도망점프.png s4=소품.png s5=걷기매달리기.png s6=걷기그루밍.png
# s1 은 5×4 칸 시트(칸 아래 이름표), 나머지는 짙은 배경에서 고양이 덩어리를 자동으로 찾아 번호 순서로 이름을 붙인다.
import os
import sys
import json
import numpy as np
from PIL import Image
from scipy import ndimage
from rembg import new_session, remove

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'sprites')
COLS = [0, 307, 614, 921, 1228, 1536]
ROWS = [(0, 258), (292, 505), (540, 752), (785, 972)]      # 이름표(알약 모양 글자)는 빼고
# 칸 순서대로 (이름, 그림 속 고양이가 보는 방향: 1 오른쪽 / -1 왼쪽 / 0 정면)
NAMES = [('walk', 1), ('headtilt', 0), ('loaf', 0), ('belly', 0), ('curious', 0),
         ('sleep', -1), ('bugcatch', -1), ('happy', 0), ('hunt', -1), ('box', 0),
         ('hugheart', 0), ('backview', 0), ('roll', -1), ('hat', 0), ('stretch', -1),
         ('peek', 1), ('run', -1), ('stargaze', -1), ('sulk', 0), ('fishtoy', 0)]


# 짙은 배경 시트: 자동으로 찾은 덩어리 번호(왼→오, 위→아래) → (이름, 보는 방향). 번호 여러 개는 한 그림으로 합친다.
# 벽·받침대가 그려진 컷(s6 의 매달리기·벽타기)은 실제 창이 벽 역할을 하므로 뺐다.
DETECTED = {
    's3': [((0,), 'run_1', -1), ((1,), 'run_2', -1), ((2,), 'run_3', -1), ((3,), 'run_4', -1),
           ((4,), 'jprep_1', -1), ((5,), 'jprep_2', -1), ((6,), 'jump_1', -1), ((7,), 'jump_2', -1),
           ((8,), 'land_1', 0), ((9,), 'land_2', 0), ((11,), 'landfail_1', 0), ((10,), 'landfail_2', 0),
           ((12,), 'roll_1', 0), ((13,), 'roll_2', 0), ((14,), 'roll_3', 0), ((15,), 'roll_4', 0)],
    's4': [((0,), 'wave', 0), ((1,), 'headphones', 0), ((2,), 'cushion', -1), ((3,), 'teacup', 0),
           ((4, 5), 'butterfly2', 1), ((6,), 'beanie', 0), ((7,), 'box2', 0), ((8,), 'yarn', -1),
           ((9,), 'stretch2', -1), ((10,), 'curious2', 0), ((11,), 'starpillow', 0), ((12,), 'dash', 1),
           ((13,), 'crown', 0), ((14,), 'backview2', 0), ((15,), 'fishhug', -1), ((16,), 'hunt2', 1),
           ((17,), 'leaf', 0), ((18,), 'shark', 0), ((20,), 'glass', 0), ((19,), 'scarf', 1)],
    's5': [((0,), 'walk_a', -1), ((1,), 'walk_b', -1), ((2,), 'walk_front', 0), ((3,), 'walk_back', 0),
           ((4,), 'hang_1', 0), ((5,), 'hang_2', 0), ((6,), 'hang_3', 0), ((7,), 'hang_wink', 0),
           ((8,), 'climb_1', -1), ((10,), 'climb_2', -1), ((9,), 'climb_3', -1), ((11,), 'climb_down', -1),
           ((12,), 'lick_a', 0), ((13,), 'wash_a', 0), ((14,), 'groombody_a', 0), ((15,), 'groomtail_a', 0),
           ((16,), 'paw_a', 0), ((17,), 'groomall', -1), ((18,), 'groomback', 0)],
    's6': [((1,), 'walk_c', -1), ((2,), 'walk_d', -1), ((3,), 'walk_happy', -1), ((4,), 'walk_back2', 0),
           ((8,), 'lick_b', 0), ((9,), 'wash_b', 0), ((10,), 'groombody_b', 0), ((11,), 'groomtail_b', 0),
           ((12,), 'paw_b', 0), ((13,), 'stretch_finish', 0)],
    # 한 줄 연속 동작(프레임) 시트 — 칸 크기를 똑같이 맞추고 여백을 자르지 않는다(넘길 때 흔들리지 않게)
    's7': [((i,), 'walk8_%d' % (i + 1), 1) for i in range(8)],
}
STRIP = {'s7'}


def _clean(a):
    """구멍 메우기 + 작은 조각 지우기."""
    solid = a[..., 3] > 128                                  # 테두리에 둘러싸인 구멍(귀 안쪽 등)은 원본 그대로 채운다
    holes = ndimage.binary_fill_holes(solid) & ~solid
    a[holes, 3] = 255
    a[a[..., 3] < 50, 3] = 0                                 # 거의 투명한 안개는 지운다
    lab, n = ndimage.label(a[..., 3] > 40)                   # 작은 조각(효과선 찌꺼기)은 지운다
    if n:
        sizes = ndimage.sum(np.ones_like(lab), lab, range(1, n + 1))
        for k, s in enumerate(sizes, 1):
            if s < 250:
                a[lab == k, 3] = 0
    return a


def iris_size(a):
    """파란 홍채 덩어리의 평균 높이(px). 시트마다 고양이를 그린 크기가 달라 이것으로 크기를 맞춘다. 눈 감은 그림은 None."""
    r, g, b, al = (a[..., i].astype(np.int32) for i in range(4))
    blue = (b > r + 40) & (b > 110) & (al > 200)
    lab, n = ndimage.label(blue)
    hs = []
    for sl in ndimage.find_objects(lab):
        h, w = sl[0].stop - sl[0].start, sl[1].stop - sl[1].start
        if 6 < h < a.shape[0] * 0.25 and 0.5 < w / h < 1.6:   # 둥근 작은 덩어리만(파란 모자·생선·상어 옷 제외)
            hs.append(h)
    hs = sorted(hs, reverse=True)[:2]
    return float(np.mean(hs)) if len(hs) == 2 else None


def _save(a, name, facing, meta, sheet):
    im = Image.fromarray(a)
    if sheet not in STRIP:
        im = im.crop(im.getbbox())
    im.save(os.path.join(OUT, name + '.png'))
    meta[name] = {'facing': facing, 'w': im.width, 'h': im.height, 'sheet': sheet, 'iris': iris_size(np.asarray(im))}
    print(name, im.size, meta[name]['iris'])


def cut_grid(path, sess, meta):
    """밝은 배경 5×4 칸 시트(첫 시트)."""
    sheet = Image.open(path).convert('RGB')
    sx, sy = sheet.width / 1536, sheet.height / 1024
    for i, (name, facing) in enumerate(NAMES):
        r, c = divmod(i, 5)
        box = (int(COLS[c] * sx), int(ROWS[r][0] * sy), int(COLS[c + 1] * sx), int(ROWS[r][1] * sy))
        src = sheet.crop(box)
        a = np.asarray(remove(src, session=sess)).copy()
        a[..., :3] = np.asarray(src)                         # 색은 원본 그대로(AI 는 투명 부분 색을 검정으로 지운다), 투명도만 쓴다
        rgb = a[..., :3].astype(np.int16)
        soft = a[..., 3] < 250
        gray = (rgb.max(-1) - rgb.min(-1)) < 28              # 분홍 귀·파란 모자는 살린다
        # 반투명 회색 = 원래 배경의 어두운 빛 번짐이 섞인 하얀 털 → 하얗게 끌어올리고 조금 더 불투명하게
        lift = soft & gray
        a[lift, :3] = (255 - (255 - rgb[lift]) * 0.25).astype(np.uint8)
        a[lift, 3] = np.minimum(255, a[lift, 3].astype(np.int16) * 3 // 2).astype(np.uint8)
        _save(_clean(a), name, facing, meta, 's1')


def detect(img):
    """짙은 배경에서 고양이 덩어리를 찾는다. (라벨 지도, 상자 목록[읽는 순서])"""
    a = np.asarray(img).astype(np.int32)
    bg = np.median(a.reshape(-1, 3), 0)
    m = np.abs(a - bg).sum(-1) > 90
    m = ndimage.binary_dilation(ndimage.binary_closing(m, iterations=4), iterations=6)
    lab, _ = ndimage.label(m)
    objs = []
    for k, sl in enumerate(ndimage.find_objects(lab), 1):
        y0, y1, x0, x1 = sl[0].start, sl[0].stop, sl[1].start, sl[1].stop
        if y1 - y0 > 90 and x1 - x0 > 70:
            objs.append((k, (x0, y0, x1, y1)))
    objs.sort(key=lambda o: (round((o[1][1] + o[1][3]) / 2 / 120), o[1][0]))
    return lab, objs, bg


def cut_detected(key, path, sess, meta):
    sheet = Image.open(path).convert('RGB')
    lab, objs, bg = detect(sheet)
    if key in STRIP:                                         # 모든 칸을 가장 큰 칸 크기로, 바닥(아래 끝) 기준 정렬
        W = max(b[2] - b[0] for _, b in objs)
        H = max(b[3] - b[1] for _, b in objs)
    for idxs, name, facing in DETECTED[key]:
        ks = [objs[i][0] for i in idxs]
        boxes = [objs[i][1] for i in idxs]
        box = (min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes))
        if key in STRIP:
            box = (box[0], box[3] - H, box[0] + W, box[3])
        src = sheet.crop(box)
        a = np.asarray(remove(src, session=sess)).copy()
        own = np.isin(lab[box[1]:box[3], box[0]:box[2]], ks)  # 이웃 칸 그림이 섞이지 않게
        a[~own, 3] = 0
        # 반투명 가장자리 색에서 짙은 배경색을 걷어낸다: 원래색 = (보인색 - (1-α)·배경) / α
        al = a[..., 3:4].astype(np.float32) / 255.0
        c = np.asarray(src).astype(np.float32)
        fg = np.clip((c - (1 - al) * bg) / np.maximum(al, 0.05), 0, 255)
        a[..., :3] = np.where(al > 0.02, fg, c).astype(np.uint8)
        # 꼬리 끝처럼 AI 가 반쯤 투명하게 잡은 하얀 털 → 더 불투명하고 하얗게
        lum = fg[..., 0] * 0.3 + fg[..., 1] * 0.6 + fg[..., 2] * 0.1
        fur = (a[..., 3] < 250) & (a[..., 3] > 20) & (lum > 170) & ((fg.max(-1) - fg.min(-1)) < 45)
        a[fur, :3] = (255 - (255 - fg[fur]) * 0.5).astype(np.uint8)
        a[fur, 3] = np.minimum(255, a[fur, 3].astype(np.int32) * 8 // 5).astype(np.uint8)
        _save(_clean(a), name, facing, meta, key)


def main(args):
    os.makedirs(OUT, exist_ok=True)
    sess = new_session('isnet-general-use')
    try:
        meta = json.load(open(os.path.join(OUT, 'sprites.json'), encoding='utf-8'))
    except (OSError, ValueError):
        meta = {}
    for arg in args:
        key, path = arg.split('=', 1)
        if key == 's1':
            cut_grid(path, sess, meta)
        else:
            cut_detected(key, path, sess, meta)
    with open(os.path.join(OUT, 'sprites.json'), 'w', encoding='utf-8') as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)


if __name__ == '__main__':
    main(sys.argv[1:])
