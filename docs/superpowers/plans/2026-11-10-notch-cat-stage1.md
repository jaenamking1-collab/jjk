# 노치 고양이 1단계 구현 계획

> 설계: `docs/superpowers/specs/2026-11-10-notch-cat-design.md` 8절 1단계. 인라인으로 실행한다.

**Goal:** 화면 위 가운데 노치 — 투명 합성, 스프링 펼침/접힘, 전체 화면 숨김, 음악(GSMTC) + 가사(LRCLIB).
**Architecture:** Rust + `windows` 크레이트. UI 스레드 하나(창·D2D·DComp, vsync Present 로 60장), 음악 스레드(GSMTC 폴링, 명령 채널), 가사 스레드(LRCLIB, 캐시).
**Tech Stack:** windows 0.58, serde_json. 외부 런타임 없음.

## Global Constraints
- 개인 데이터는 `%APPDATA%\notch-cat\` 에만. 로그 `log.txt` 최근 500줄.
- 평소 메모리 50MB 이하, 쉴 때 CPU 1% 아래, 움직임은 전부 스프링.
- 글꼴 맑은 고딕. 생존 표시: 펼친 화면 구석 "음악 hh:mm · 가사 hh:mm", 10분 넘으면 경고색.

## 파일
| 파일 | 일 |
|---|---|
| `notch-cat/src/spring.rs` | 감쇠 용수철 + 시험 |
| `notch-cat/src/lrc.rs` | LRC 해석·현재 줄·제목 정리 + 시험 |
| `notch-cat/src/media.rs` | GSMTC 폴링·명령(재생/멈춤/넘기기/탐색)·앨범 그림 바이트 |
| `notch-cat/src/lyrics.rs` | LRCLIB `/api/get`→`/api/search`, 캐시 |
| `notch-cat/src/gfx.rs` | D3D11·DXGI 합성 스왑체인·DComp·D2D·DWrite·WIC |
| `notch-cat/src/main.rs` | 창·트레이·루프·그리기·클릭·숨김·로그·설정 |
| `.github/workflows/notch-cat-exe.yml` | cargo test → release → 20초 시험 → 릴리스 `notch` |

## 작업
1. spring/lrc 시험 먼저 → 통과 (`cargo test`)
2. gfx + main: 빈 알약이 뜨고 펼침/접힘, 클릭 통과 확인
3. media + lyrics 붙이고 실제 곡으로 대조
4. 워크플로 → 릴리스 → 링크
5. 검증: 재생기 ↔ 노치 사진, Nusa 나란히, 메모리·CPU
