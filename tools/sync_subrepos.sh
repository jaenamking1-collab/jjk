#!/usr/bin/env bash
# jjk 안의 하위 프로젝트(자기 .git 을 가진 폴더)를 모두 커밋·받기·올리기.
# 세션 시작(pull 먼저)과 턴 끝(Stop 훅)에서 돈다. jjk 본체 훅은 이 폴더들을 건드리지 못한다.
# ponytail: 폴더마다 순서대로 돈다. 하위 저장소가 많아져 훅이 시간 초과되면 병렬로.
cd "$(dirname "$0")/.." || exit 0
bad=""
for g in */.git; do
  d="${g%/.git}"
  [ -e "$d/.git/MERGE_HEAD" ] || [ -d "$d/.git/rebase-merge" ] || [ -d "$d/.git/rebase-apply" ] && { bad="$bad $d(병합중)"; continue; }
  git -C "$d" symbolic-ref -q HEAD >/dev/null || continue
  if [ -n "$(git -C "$d" status --porcelain)" ]; then
    git -C "$d" add -A && git -C "$d" commit -qm "auto-sync $(date +%Y-%m-%d_%H:%M:%S)" >/dev/null 2>&1
  fi
  git -C "$d" pull -q --rebase --autostash origin HEAD >/dev/null 2>&1 || { git -C "$d" rebase --abort >/dev/null 2>&1; bad="$bad $d(받기실패)"; continue; }
  git -C "$d" push -q origin HEAD >/dev/null 2>&1 || bad="$bad $d(올리기실패)"
done
if [ -n "$bad" ]; then
  printf '{"systemMessage":"⚠️ 클라우드에 못 올린 프로젝트:%s — 이 기기에만 있습니다."}' "$bad"
fi
exit 0
