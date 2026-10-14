"""공식 모드 목록(anthropics/claude-plugins-official)에서 새로 올라온 것을 찾아 new.txt 에 덧붙인다.
매일 new-mods.yml 이 돈다. AI 를 안 써서 사용량이 들지 않는다. 사람 알림은 없고,
jjk-guard 할 일 띠가 '🆕 새 모드 N개'로 보여준다 → 사용자가 '새 모드 봐줘'라고 하면 골라 추천한 뒤 비운다."""
import json, os, sys, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
URL = 'https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/.claude-plugin/marketplace.json'


def main():
    data = json.load(urllib.request.urlopen(URL, timeout=30))
    seen_p, new_p = os.path.join(HERE, 'seen.txt'), os.path.join(HERE, 'new.txt')
    seen = set(open(seen_p, encoding='utf-8').read().split())
    fresh = [p for p in data['plugins'] if p['name'] not in seen]
    if not fresh:
        print('새 모드 없음'); return
    with open(new_p, 'a', encoding='utf-8') as f:
        for p in fresh:
            f.write('%s — %s\n' % (p['name'], ' '.join(p.get('description', '').split())[:160]))
    with open(seen_p, 'w', encoding='utf-8') as f:
        f.write('\n'.join(sorted(seen | {p['name'] for p in fresh})) + '\n')
    print('새 모드 %d개: %s' % (len(fresh), ', '.join(p['name'] for p in fresh)))


if __name__ == '__main__':
    sys.exit(main())
