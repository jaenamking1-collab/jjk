# 그림 시트(5×4 칸, 칸 아래에 이름표)를 동작별 투명 PNG 로 오린다. 그림을 새로 받았을 때만 돌린다.
#   pip install "rembg[cpu]" scipy   →   python make_sprites.py 시트.png
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


def main(path):
    os.makedirs(OUT, exist_ok=True)
    sheet = Image.open(path).convert('RGB')
    sx, sy = sheet.width / 1536, sheet.height / 1024
    sess = new_session('isnet-general-use')
    meta = {}
    for i, (name, facing) in enumerate(NAMES):
        r, c = divmod(i, 5)
        box = (int(COLS[c] * sx), int(ROWS[r][0] * sy), int(COLS[c + 1] * sx), int(ROWS[r][1] * sy))
        src = sheet.crop(box)
        cut = remove(src, session=sess)
        a = np.asarray(cut).copy()
        a[..., :3] = np.asarray(src)                         # 색은 원본 그대로(AI 는 투명 부분 색을 검정으로 지운다), 투명도만 쓴다
        solid = a[..., 3] > 128                              # 테두리에 둘러싸인 구멍(귀 안쪽 등)은 원본 그대로 채운다
        holes = ndimage.binary_fill_holes(solid) & ~solid
        a[holes, 3] = 255
        rgb = a[..., :3].astype(np.int16)
        soft = a[..., 3] < 250
        gray = (rgb.max(-1) - rgb.min(-1)) < 28                # 분홍 귀·파란 모자는 살린다
        # 반투명 회색 = 원래 배경의 어두운 빛 번짐이 섞인 하얀 털 → 하얗게 끌어올리고 조금 더 불투명하게
        lift = soft & gray
        a[lift, :3] = (255 - (255 - rgb[lift]) * 0.25).astype(np.uint8)
        a[lift, 3] = np.minimum(255, a[lift, 3].astype(np.int16) * 3 // 2).astype(np.uint8)
        a[soft & (a[..., 3] < 50), 3] = 0                    # 거의 투명한 안개는 지운다
        lab, n = ndimage.label(a[..., 3] > 40)               # 작은 조각(효과선 찌꺼기)은 지운다
        if n:
            sizes = ndimage.sum(np.ones_like(lab), lab, range(1, n + 1))
            for k, s in enumerate(sizes, 1):
                if s < 250:
                    a[lab == k, 3] = 0
        im = Image.fromarray(a)
        im = im.crop(im.getbbox())
        im.save(os.path.join(OUT, name + '.png'))
        meta[name] = {'facing': facing, 'w': im.width, 'h': im.height}
        print(name, im.size)
    with open(os.path.join(OUT, 'sprites.json'), 'w', encoding='utf-8') as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)


if __name__ == '__main__':
    main(sys.argv[1])
