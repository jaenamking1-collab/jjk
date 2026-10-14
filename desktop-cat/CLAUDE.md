# desktop-cat/ — 바탕화면 고양이 (윈도우 전용 파이썬, 포트폴리오와 무관)

> 2026-10-14 새로 만듦(WORKLOG 234·238·239).

- 무엇: 윈도우 바탕화면에 사는 고양이 위젯. 그림은 `sprites/`, 동작은 `cat_brain.py`, 그리기는 `cat_render.py`·`sprite_render.py`, 창 처리 `win_world.py`.
- exe: `desktop-cat-exe.yml` 이 main 푸시마다 PyInstaller 로 만들고, 20초 실행 시험(생존·움직임·오류 없음)을 통과해야 릴리스 `cat` 에 올라간다.
- 영상(Seedance) 계획·입력 그림: `VIDEO_TODO.md`, `video_in/`. 영상 생성 비용은 Claude 요금제가 내지 않는다.
- 이 컨테이너엔 윈도우·tkinter 가 없다 — 시험은 `preview.py`·`wintest.py` 방식으로, 됐다고 하기 전에 무엇을 어떻게 확인했는지 밝힌다.
