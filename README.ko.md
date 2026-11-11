<div align="center">

# RiverLens

**당신의 핸드, 당신의 데이터. 로컬 완결 포커 리뷰 워크스페이스.**

20개 이상의 룸에서 완료된 자신의 핸드 히스토리를 가져오고, 집단 성향을 연구하고, 실제 증거로 누수를 찾고, 연습으로 체화하세요 — 모두 오프라인으로 동작합니다.

[![Release](https://img.shields.io/github/v/release/G-Loop-co/riverlens)](https://github.com/G-Loop-co/riverlens/releases/latest)
[![Desktop CI](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml/badge.svg)](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%28Apple%20Silicon%20%26%20Intel%29-blue)
![Runtime](https://img.shields.io/badge/runtime-offline%20first-2f6f4f)

**[⬇️ RiverLens v0.6.1 다운로드](https://github.com/G-Loop-co/riverlens/releases/latest)**

<sub>macOS 빌드는 ad-hoc 서명입니다(notarization 없음).</sub>

![RiverLens overview — synthetic data](docs/screenshots/overview-en.jpg)

</div>


[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

---

## 기능

- TXT, ZIP, 폴더 가져오기. 진행률, 일시 중지·재개, 중복 탐지와 이상 기록 격리.
- 순손익, bb/100, 세션별·포지션별 통계에서 개별 핸드 확인.
- VPIP, PFR, RFI, 3-bet, 블라인드 방어 등 핵심 통계 13개. 분자와 기회 분모를 각각 확인.
- 13 × 13 시작 핸드 매트릭스에서 표본 수, net bb, bb/100, 액션 빈도 확인.
- 액션별 재생, 스트리트 이동, 알려진 카드 표시.
- 메모, 태그, 복기 완료 상태와 필터 저장.
- 로컬 SQLite, CSV／핸드 기록 내보내기, 백업 및 복원.
- Forest, Midnight, Paper(따뜻한 흰색) 테마와 로컬 선택 저장.
- 지원되는 헤즈업·단일 팟·알려진 홀카드·단일 runout의 all-in equity. decision EV나 GTO 점수가 아닙니다.

## 스크린샷

Paper 테마를 1920px 너비의 전체 페이지로 캡처했습니다. 합성 핸드 240개만 사용하며 개인 기록은 없습니다. 반복 표본의 빈도와 수익은 실제 성적이 아닙니다. 매트릭스는 관측된 핸드를 보여주며 추천 레인지가 아닙니다.

![RiverLens — Paper](docs/screenshots/overview-en.jpg)

![RiverLens — 13 × 13](docs/screenshots/starting-hands-en.jpg)

## 시작하기

Node.js 22+, Rust stable 및 Tauri 플랫폼 필수 도구가 필요합니다. macOS는 Xcode Command Line Tools, Windows는 MSVC C++ Build Tools와 WebView2를 사용합니다.

[Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. PokerCraft에서 본인의 종료된 핸드 기록을 TXT／ZIP으로 내보냅니다.
2. Data & settings에서 브랜드, Hero 이름, 기록 본문의 시간대를 확인합니다.
3. Import center에서 가져온 뒤 통계, 매트릭스, 리플레이를 살펴봅니다.
4. 메모를 저장하고 정기적으로 백업합니다.

## 개발

터미널 1에서 Rust core, 터미널 2에서 브라우저 미리보기를 실행합니다. 데스크톱은 Tauri IPC와 네이티브 파일 대화상자를 사용합니다. 미리보기는 같은 Rust core를 사용하며 개발용 경로 입력을 제공합니다.

```sh
# Terminal 1
npm run serve:core
# Terminal 2 — http://127.0.0.1:1420
npm run dev
```

```sh
npm test
npm run core:test
npm run build
node scripts/cargo.mjs clippy -p poker-core --all-targets -- -D warnings
npm run desktop:build -- --bundles app
# Windows
npm run desktop:build -- --target x86_64-pc-windows-msvc --bundles nsis
```

데이터는 Tauri app-data(app.riverlens.desktop)에 저장되며 설정에서 실제 경로를 확인할 수 있습니다. 브라우저 개발은 .local/riverlens.db를 사용합니다. 실행 중인 SQLite는 WAL까지 처리하는 내장 백업 기능을 이용하세요.

## 범위와 라이선스

개인 오프라인 세션 종료 후 복기용입니다. 게임 클라이언트 연결, 실시간 HUD, RTA, 클라우드 동기화, GTO 최선 액션 평가는 없습니다. Natural8／GGPoker와 제휴하지 않습니다. 공개 저장소이지만 프로젝트 전체 재사용 라이선스는 아직 정하지 않았습니다. RiverLens 자체를 MIT／Apache 라이선스로 간주하지 마세요. 타사 구성 요소는 각 라이선스를 유지합니다.

## 문서

[User guide — 繁體中文](docs/user-guide.md) · [Architecture](docs/architecture.md) · [Validation](docs/validation.md) · [Research](docs/research-matrix.md) · [Changelog](CHANGELOG.md) · [Third-party components](THIRD_PARTY.md) · [Releases](https://github.com/G-Loop-co/riverlens/releases)
