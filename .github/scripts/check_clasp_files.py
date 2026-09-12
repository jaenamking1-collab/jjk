#!/usr/bin/env python3
"""`clasp status` 출력을 읽어 **Code.gs 외의 파일이 올라가려 하면 실패**시킨다.

왜: 이 저장소에는 Apps Script 파일이 여러 개 있는데 서로 **다른 프로젝트**다.
  · Code.gs           → 포트폴리오관리 (이 워크플로가 배포하는 대상)
  · okx_nft_alert.gs  → 별개 프로젝트 (30분 트리거가 포트폴리오 백엔드의 실행 슬롯을 훔친다)
  · public_dist_proxy.gs → 별개 프로젝트
  · stock_sheet.gs    → '자산' 스프레드시트에 붙은 별개 프로젝트 (계정도 다르다)
`.claspignore` 가 이걸 막게 돼 있지만, 그게 깨진 채로 push 하면 전부 한 프로젝트에 섞인다.
2026-09-12 첫 배포 시도에서 `clasp status` 에 tools/·WORKLOG.md 가 보여 확인 장치를 넣었다.

사용법: clasp status | python3 .github/scripts/check_clasp_files.py
"""
import re
import sys

ALLOWED = {'Code.gs', 'appsscript.json'}


def main():
    txt = sys.stdin.read()
    print(txt)

    # clasp 2.x/3.x 모두 'Not ignored files:' 다음에 실제로 올라갈 목록을 찍는다.
    m = re.search(r'Not ignored files:(.*?)(?:Ignored files:|\Z)', txt, re.S)
    if not m:
        print('::warning::clasp status 형식을 못 읽었다 — 수동 확인이 필요하다.')
        return 0

    files = []
    for line in m.group(1).splitlines():
        s = line.strip()
        if not s:
            continue
        s = s.lstrip('└├─│ ').strip()      # 트리 문자 제거
        if s:
            files.append(s)

    print('올라갈 파일:', files or '(없음)')
    bad = [f for f in files if f not in ALLOWED]
    if bad:
        print('::error::Code.gs 외의 파일이 올라가려 한다: %s' % bad)
        print('::error::.claspignore 가 깨졌다. 다른 Apps Script 프로젝트가 섞이므로 push 를 중단한다.')
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
