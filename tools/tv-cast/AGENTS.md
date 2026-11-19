# TV 자막 전송 작업 규칙

- 파일 선택은 브라우저 기본 input[type=file]로 처리한다. 숨겨진 PowerShell 대화상자는 실제 화면에서 안 뜬 사례가 있으므로 사용하지 않는다. 파일은 localhost로 스트리밍하여 임시 폴더에 보관한다.

- Windows 실행.cmd는 ASCII 내용과 CRLF 줄바꿈을 유지한다(.gitattributes). node 테스트만으로 실행 확인을 대신하지 말고 cmd.exe로 실행.cmd --check 및 실제 시작을 검증한다.

- 원본: 사용자가 선택한 로컬 영상·외부 자막. 결과: LG TV 화면.
- 실제 FFmpeg 합성 영상의 자막 픽셀과 HTTP 범위 요청·SOAP 응답을 검증한다.
- SOAP 200이나 TV의 파일 요청을 실제 자막·소리 검증으로 표현하지 않는다.
- 영상·자막·개인 경로·TV IP·실행 로그를 공개 저장소에 넣지 않는다. 캐시는 OS 임시 폴더만 쓴다.
- 로컬 UI는 loopback 및 세션 헤더로 제한하고 LAN에는 난수 경로의 영상 한 파일만 제공한다.
- 화면에 진행률·프로그램 생존 표시·TV 요청 시각·오류를 유지한다. 서버·변환은 종료 시 정리한다.
- FFmpeg 8.1의 SMI 빈 종료 구간 디코딩 오류를 재현했다. SMI는 먼저 SRT로 정규화하고 한국어 선택·종료 시각을 검증한다.
- 중지 때 SOAP 취소·변환 종료·서버 정리까지 새 재생을 차단한다. 이전 전송의 감시 타이머는 새 전송 전에 해제한다.
- TV가 다른 앱 재생·일시정지 상태면 LG가 SetAVTransportURI를 500 "Transition not available"로 거절한다(실제 UK6570PUB에서 확인). play()는 먼저 Stop → GetTransportInfo로 STOPPED 확인 후 SetURI한다.
- 영화 파일 인자(탐색기 보내기)로 실행하면 같은 이름 자막 자동 선택·TV 1대면 자동 재생. 재생은 변환 완료를 기다리지 않는 라이브 MPEG-TS(startLive, 시크 불가). UMS(Universal Media Server)는 LG에 원본을 그대로 보내 TV 내장 영어 자막이 나와 포기.
- 넘기기: 화면 버튼(/seek)이 그 위치부터 스트림을 새로 시작(-ss + setpts 로 자막 시각 보정). LG UK6570PUB 은 SOAP Seek 를 200으로 받고도 라이브 스트림에선 실제로 넘기지 않음(TimeSeekRange 요청도 안 옴) → TV 리모컨 넘기기는 안 됨(2026-11-19 확인).
- 기본 재생 = remux(영상 재압축 없이 한국어 SRT 를 mkv 안에 넣음, 2.5GB 8초) + 파일 서버(video/x-mkv, contentFeatures/DIDL 에 DLNA.ORG_OP=01·FLAGS). OP=01 과 mime 이 빠지면 LG 가 범위 요청을 안 해 넘기기가 안 된다(2026-11-19 확인). h264/hevc 가 아니면 startLive 로 대체.
