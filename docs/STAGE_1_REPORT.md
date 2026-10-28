# 1단계 완료 기록

작업일: 2026-10-28. Tauri 2 + React + TypeScript + Vite + Rust 기본 앱을 구성하고 Windows에서 실행했다. 2단계는 승인 대기이며 API 키 등록, DB, API 클라이언트 기능은 구현하지 않았다. 외부 maplemate 스킬은 사용하거나 참고하지 않았다.

## 프로젝트 구성

표시명은 Maple Ledger, 식별자는 `com.mapleledger.app`이다. 기존 폴더에 최소 파일을 직접 구성하여 생성 도구의 기존 파일 덮어쓰기를 피했다. React 진입점과 app 디렉터리, Rust 실행 진입점과 라이브러리, Tauri 창·보안 설정을 분리했다. 향후 설정 UI, API 클라이언트, DB 저장소는 ARCHITECTURE에 정의한 위치에 해당 단계에서 추가한다. 빈 도메인 모듈이나 DB를 미리 생성하지 않았다.

현재 IPC 기능은 없고 capability 권한은 빈 목록이다. Vite는 `127.0.0.1:1420`에서만 실행하며 `secrets` 및 Git 메타데이터 제공을 차단한다. 배포 bundle은 비활성화했고 별도 resources 목록을 추가하지 않았다. 아이콘은 단순 M 글자의 개발용 기본 아이콘이다.

## 설치 의존성과 환경

- React/ReactDOM 19.3.1, TypeScript 5.9.3, Vite 6.4.5, React 플러그인 4.7.0, React 타입 19.3.0, Tauri CLI 2.12.4.
- Rust 직접 의존성은 tauri 2.12.5와 tauri-build 2.7.4다. 나머지 Rust 패키지는 이들의 전이 의존성이다. reqwest, SQLite 드라이버, 키 저장 어댑터, 전역 상태 및 테스트 프레임워크는 추가하지 않았다.
- Node 22.15.0/npm 10.9.2를 그대로 사용했다. Vite 6.4.5의 Node 엔진 범위와 React 플러그인 호환 범위를 npm 메타데이터에서 확인했다. 전역 Node/npm 업그레이드는 하지 않았다.
- Rust/Cargo 1.99.1 및 `x86_64-pc-windows-msvc` 설치를 확인했다. 사용자 PATH에 기존 경로를 보존하며 `%USERPROFILE%\.cargo\bin`만 추가했다. 기존에 열린 터미널/편집기에는 재시작이 필요할 수 있다. 이번 개발 프로세스에는 같은 경로를 명시적으로 적용했다.
- Visual Studio 2022 Community C++ 도구, 기존 WebView2 Runtime 155.0.4283.62를 이용했다. 실제 Rust 링크와 창 렌더링 성공으로 개발 환경 동작을 확인했다. 도구 재설치는 하지 않았다.

## 검증 결과

| 검증 | 결과 |
| --- | --- |
| npm install | 필요한 71개 npm 패키지 설치, package-lock.json 생성 |
| npm run typecheck | React 및 Vite 설정 타입 검사 통과 |
| cargo fmt --manifest-path src-tauri/Cargo.toml -- --check | 통과 |
| npm run tauri dev | 최초 dev 컴파일 약 1분 19초, target/debug/maple-ledger.exe 실행 성공 |
| 개발 서버 및 App.tsx | HTTP 200, 제목과 React 모듈 확인 |
| 실제 Windows UI | 창 제목 Maple Ledger, 접근성 트리에서 제목·한글 문구 확인, 해당 앱 창만 캡처하여 정상 렌더링 확인 |
| npm audit | 개발 의존성 포함 보고된 취약점 0개; Rust 의존성 취약점 스캔은 미수행 |
| Git 제외 규칙 | 키, node_modules, dist, target, gen, SQLite 및 WAL/SHM 경로 제외 확인 |

화면 재확인은 이미 컴파일한 개발 실행 파일과 Vite 서버를 사용하여 추가 Cargo 빌드를 하지 않았다. 디버그 실행 파일은 콘솔 하위 시스템이므로 WaitForInputIdle 방식이 적합하지 않아 실제 창 핸들과 접근성 트리로 확인했다. 확인용 캡처·로그와 임시 실행 프로세스는 종료 시 정리한다. Cargo 캐시 및 node_modules는 반복 다운로드·빌드를 피하도록 유지하고 Git에서는 제외한다.

프런트엔드 production 빌드, release 빌드, 설치 패키지, 실제 넥슨 API 호출, 키 저장, SQLite 및 기능 테스트는 이번 범위가 아니므로 수행하지 않았다. API 호출은 0회다. Git 커밋과 푸시도 하지 않았다.

## 파일 변경

수정: `.gitignore`, `AGENTS.md`, `docs/ARCHITECTURE.md`, `docs/DEVELOPMENT_PLAN.md`.

신규:
- `README.md`, `docs/STAGE_1_REPORT.md`
- `package.json`, `package-lock.json`, `index.html`
- `tsconfig.json`, `tsconfig.node.json`, `vite.config.ts`
- `src/main.tsx`, `src/app/App.tsx`, `src/app/styles.css`
- `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/build.rs`
- `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/icons/icon.ico`

이전 단계에서 생성되어 아직 Git 미추적 상태였던 문서는 그대로 유지했다. 원본 API 레퍼런스와 개발 키 파일은 수정하지 않았다. 개발 키 내용은 읽거나 출력하지 않았다. 확정된 본캐 1개 필수·본캐 포함 최대 6개·동일 계정/월드·충돌 확인 후 재설정·기존 기록 보존 정책은 AGENTS와 설계·개발 계획에 반영했다.

## 다음 단계

사용자 승인 후 2단계에서 키 보호 방식(Windows Credential Manager 또는 DPAPI)과 저장소 구성을 확정하고 설정 UI, 키 등록·교체·삭제·재시작 복원을 구현한다. 필요한 의존성과 대안을 제시한 뒤 도입한다. 현재 이 단계로 넘어가지 않는다.

참고: [Tauri 프로젝트 구성](https://v2.tauri.app/start/create-project/), [Windows 개발 사전 준비](https://v2.tauri.app/start/prerequisites/).
