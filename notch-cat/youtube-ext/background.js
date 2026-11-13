// 유튜브 탭이 보낸 목록을 노치 고양이(이 PC 안)로 넘기고, 노치가 돌려준 "이 곡 틀어"를 탭에 전한다.
chrome.runtime.onMessage.addListener((msg, sender, reply) => {
  const body = JSON.stringify({ ...msg, tab: sender.tab ? sender.tab.id : 0 });
  fetch('http://127.0.0.1:47811/yt', { method: 'POST', body })
    .then(r => r.json())
    .then(j => reply(j))
    .catch(() => reply({}));
  return true;
});
