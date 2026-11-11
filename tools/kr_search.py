"""국내 자료 검색: 네이버 검색 결과와 네이버 블로그 본문을 글자로 받는다.
포트폴리오(Code.gs)가 쓰는 방식 그대로: 브라우저 말고 직접 받아서 글자만 꺼낸다.

  python kr_search.py "엔트리 티처블머신"            # 블로그 검색 (기본)
  python kr_search.py "엔트리 티처블머신" --where web  # 통합검색(web)·cafe·news 도 됨
  python kr_search.py read https://blog.naver.com/ID/글번호
"""
import html, re, sys, urllib.parse, urllib.request

UA = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                    "(KHTML, like Gecko) Chrome/130.0 Safari/537.36"}


def get(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=20) as r:
        return r.read().decode("utf-8", "replace")


def text(s):
    s = re.sub(r"<(script|style)\b.*?</\1>", " ", s, flags=re.S)
    return re.sub(r"[\s​]+", " ", html.unescape(re.sub(r"<[^>]+>", " ", s))).strip()


def search(q, where="blog", n=10):
    page = get("https://search.naver.com/search.naver?where=%s&query=%s" % (where, urllib.parse.quote(q)))
    found = {}
    for href, inner in re.findall(r'<a[^>]+href="(https?://[^"]+)"[^>]*>(.*?)</a>', page, re.S):
        if "naver.com/" in href and any(k in href for k in ("search.naver", "keep.naver", "help.naver")):
            continue
        t = text(inner).replace("새 창 열림", "").strip()
        if len(t) > 3:
            found.setdefault(href, []).append(t)
    out = []
    for href, ts in found.items():
        ts = sorted(set(ts), key=len, reverse=True)
        if len(ts[0]) >= 8 and re.search(r"/\d{6,}|\.(com|kr|net|org)/.+", href):
            out.append((href, ts[0], ts[1] if len(ts) > 1 else ""))
    return out[:n]


def read(url):
    m = re.search(r"blog\.naver\.com/([\w-]+)/(\d+)", url) or \
        re.search(r"blogId=([\w-]+).*?logNo=(\d+)", url)
    if m:  # 네이버 블로그는 본문 틀(PostView)을 직접 받는다
        url = "https://blog.naver.com/PostView.naver?blogId=%s&logNo=%s&redirect=Dlog&widgetTypeCall=true" % m.groups()
    page = get(url)
    i = page.find("se-main-container")
    return text(page[i:i + 400000] if i >= 0 else page)


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    if a[0] == "read":
        print(read(a[1])[:20000])
    else:
        where = a[a.index("--where") + 1] if "--where" in a else "blog"
        for href, title, snip in search(a[0], where):
            print("- %s\n  %s\n  %s\n" % (title, href, snip[:200]))
