#!/usr/bin/env python3
"""백엔드(Apps Script)가 살아있는지 바깥에서 확인한다.

왜 저장소에 있나: 기존 감시자 distWatchdog 는 Apps Script 안에 살아서, 트리거가 죽으면
감시자도 같이 죽었다 — 2026-08-29 이후 일일 점검 메일이 끊긴 게 그 때문이고, 그래서
snapshotPrices 가 8/19부터 3주간 죽어 있는 걸 아무도 몰랐다(WORKLOG 149).
감시자는 감시 대상 밖에 있어야 한다.

앱 화면에도 🚨 배너가 있지만 그건 사람이 앱을 열어야 보인다. 이건 안 열어도 알려준다.

출력은 GITHUB_OUTPUT 형식(key=value) 한 줄씩. 종료코드는 항상 0 — 판정 결과는 status 로 낸다.
토큰이 필요 없다: getDistributionAll 은 PUBLIC_ACTIONS 라 공개다.

로컬 테스트: MOCK_SAVED_AT='2026-09-10 15:16' python3 .github/scripts/check_backend.py
"""
import datetime
import json
import os
import sys
import urllib.request

API = ('https://script.google.com/macros/s/'
       'AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s/exec')
KST = datetime.timezone(datetime.timedelta(hours=9))


def limit_hours(day):
    """임계는 백엔드가 실제로 도는 주기에 맞춘다(Code.gs 의 _inNoticeWindow 와 같은 날짜 조건).

    공지창(8~12일, 23일~)이면 checkDistNotices 가 09~18시에 30분마다 도니 6시간이면 죽은 것이고,
    그 밖의 날은 refreshAllDistributions 가 새벽 5시 1회뿐이라 30시간까지는 정상이다.
    """
    return 6 if ((8 <= day <= 12) or day >= 23) else 30


def fetch_saved_ats():
    """운용사별 savedAt 목록. 접속 실패는 예외로 올린다."""
    mock = os.environ.get('MOCK_SAVED_AT')
    if mock:                                    # 로컬 테스트용 — 워크플로에서는 설정하지 않는다
        return [mock] * 6
    raw = urllib.request.urlopen(API + '?action=getDistributionAll', timeout=120).read()
    src = (json.loads(raw) or {}).get('sources') or {}
    return sorted(v.get('savedAt') for v in src.values() if v.get('savedAt'))


def main():
    now = datetime.datetime.now(KST)
    limit = limit_hours(now.day)
    out = {'limit': limit}

    try:
        times = fetch_saved_ats()
    except Exception as e:                      # 접속 자체가 안 되는 것도 '정지'다
        out['status'] = 'unreachable'
        out['detail'] = '백엔드에 접속할 수 없습니다 — %s' % str(e)[:200].replace('\n', ' ')
    else:
        if not times:
            out['status'] = 'nodata'
            out['detail'] = '응답은 왔지만 savedAt 이 하나도 없습니다'
        else:
            last = times[-1]
            saved = datetime.datetime.strptime(last, '%Y-%m-%d %H:%M').replace(tzinfo=KST)
            hours = int((now - saved).total_seconds() // 3600)
            out['status'] = 'stale' if hours >= limit else 'ok'
            out['hours'] = hours
            out['detail'] = ('마지막 갱신 %s (%d시간 전) · 기준 %d시간 · 운용사 %d곳'
                             % (last, hours, limit, len(times)))

    dest = os.environ.get('GITHUB_OUTPUT')
    lines = ['%s=%s' % (k, v) for k, v in out.items()]
    if dest:
        with open(dest, 'a') as f:
            f.write('\n'.join(lines) + '\n')
    print('\n'.join(lines))
    return 0


if __name__ == '__main__':
    sys.exit(main())
