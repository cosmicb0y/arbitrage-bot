---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-03-success', 'step-04-journeys', 'step-05-domain', 'step-06-innovation', 'step-07-project-type', 'step-08-scoping', 'step-09-functional', 'step-10-nonfunctional', 'step-11-polish']
status: complete
classification:
  projectType: desktop_app
  domain: fintech
  complexity: high
  projectContext: brownfield
  featureScope: bloomberg-terminal-wts
panels:
  - name: console
    description: 실행 로그, 시스템 메시지, 에러 표시
  - name: exchange_tabs
    description: 6개 거래소 전환 (Binance, Coinbase, Bybit, GateIO, Upbit, Bithumb)
  - name: balance
    description: 선택된 거래소의 자산별 잔고/포지션
  - name: orderbook
    description: 선택된 마켓의 호가창
  - name: trade
    description: 주문 입력 (시장가/지정가/Stop-Limit/OCO 등)
  - name: deposit_withdraw
    description: 입금 주소 생성, 출금 요청
  - name: calculator_memo
    description: USD/KRW 환율 계산기, 사용자 메모
  - name: exchange_links
    description: 각 거래소 웹사이트 바로가기
inputDocuments:
  - 'docs/index.md'
  - 'docs/project-overview.md'
  - 'docs/architecture.md'
  - 'docs/api-contracts.md'
  - 'docs/development-guide.md'
  - 'docs/source-tree-analysis.md'
workflowType: 'prd'
documentCounts:
  briefs: 0
  research: 0
  brainstorming: 0
  projectDocs: 6
---

# Product Requirements Document - arbitrage-bot

**Feature:** Bloomberg Terminal Style Web Trading System (WTS)
**Author:** Hyowon
**Date:** 2026-01-18

---

## Executive Summary

### 문제 정의

현재 암호화폐 거래 시 각 거래소 웹사이트에 개별 접속해야 하며, 거래소 프론트엔드 장애 시 거래가 불가능합니다. 6개 거래소를 관리하려면 여러 브라우저 탭을 오가며 작업해야 합니다.

### 솔루션

블룸버그 터미널 스타일의 Web Trading System(WTS)을 구현합니다. 단일 창에서 6개 거래소의 거래, 입출금, 잔고 관리를 수행할 수 있으며, 거래소 웹사이트 장애와 독립적으로 API를 통해 직접 거래합니다.

### 핵심 가치

| 항목 | 설명 |
|------|------|
| **비즈니스 가치** | 거래소 웹 방문 없이 통합 거래, 장애 독립성 |
| **기술적 범위** | Tauri 새 창, 8개 패널, 6개 거래소 API 연동 |
| **복잡도** | 높음 (Fintech, 거래소별 API 차이, 실시간성) |

---

## Success Criteria

### User Success

| 기준 | 목표 |
|------|------|
| 거래소 웹 방문 없이 거래 | 100% 가능 (거래소 프론트 장애 시에도 동작) |
| 주문 실행 | API 응답 시간 내 완료 (거래소별 상이) |
| 입출금 주소 발행 | 정상 발행, 즉시 확인 가능 |
| 출금 실행 | 주소/수량 입력 → 거래소 API 통해 문제없이 처리 |
| 패널 UX | 8개 패널 동시 표시, 거래소 전환 시 즉시 반영 |

### Technical Success

| 기준 | 목표 |
|------|------|
| 안정성 | 최대한 장시간 무중단 운영 |
| 에러 처리 | 거래 실패 시 사용자에게 명확히 알림 (콘솔 + UI) |
| API 장애 대응 | 거래소 API 장애 감지 및 표시 |
| 거래소 프론트 독립 | 거래소 웹 다운 시에도 API 통해 정상 동작 |
| 실시간성 | 내부 배치 처리 금지, 모든 요청/응답 즉시 처리 |
| UI 반응성 | 사용자 액션 → 즉각적 피드백 (배칭 없음) |

### 실시간성 세부 요구사항

| 항목 | 요구사항 |
|------|----------|
| 주문 요청 | 버튼 클릭 즉시 API 호출 (큐잉/배치 금지) |
| 잔고 조회 | 요청 즉시 조회 (캐시 최소화) |
| 오더북 갱신 | WebSocket 실시간 스트리밍 |
| 입출금 요청 | 즉시 API 호출 |
| 콘솔 로그 | 이벤트 발생 즉시 표시 |

### Business Success

| 기준 | 목표 |
|------|------|
| 거래 편의성 | 거래소 웹 대비 동등 이상의 기능 |
| 운영 효율 | 단일 창에서 6개 거래소 관리 |

### Measurable Outcomes

- 주문 성공률: API 정상 시 100%
- 입출금 성공률: API 정상 시 100%
- 에러 발생 시 사용자 알림: 100%

---

## Product Scope

### MVP - Minimum Viable Product

**우선 지원: Upbit**
- 잔고 조회
- 오더북 표시
- 매수/매도 주문 (시장가, 지정가)
- 입금 주소 생성
- 출금 요청
- 콘솔 로그
- 환율 계산기
- 거래소 간편 링크

### Growth Features (Phase 2)

**추가 거래소 확장:**
- Bithumb
- Binance
- Bybit
- GateIO
- Coinbase

**주문 유형 확장:**
- Stop-Limit
- OCO

### Vision (Future)

- 차익거래 시나리오 자동 실행
- 멀티 거래소 동시 주문
- 거래 히스토리 분석

---

## User Journeys

### Journey 1: 일반 매수/매도 - 성공 시나리오

**상황:** Upbit에서 BTC를 매수하려고 함

**시스템 흐름:**
1. WTS 창 열기 (모니터링 앱과 별도 창)
2. 거래소 탭에서 Upbit 선택
3. 잔고 패널에서 KRW 잔고 확인
4. 오더북 패널에서 BTC/KRW 호가 확인
5. 매수/매도 패널에서 수량, 가격(지정가) 또는 시장가 선택
6. 매수 버튼 클릭 → 즉시 API 호출 (배치 없음)
7. 콘솔에 주문 결과 표시 (성공/실패)
8. 잔고 패널 갱신 (BTC 증가, KRW 감소)

**결과:** 거래소 웹 방문 없이 매수 완료

### Journey 2: 입금 주소 생성

**상황:** 외부에서 ETH를 받기 위해 Upbit 입금 주소 필요

**시스템 흐름:**
1. 거래소 탭에서 Upbit 선택
2. 입출금 패널 열기
3. 입금 탭 선택 → ETH 선택 → 네트워크 선택 (ERC20 등)
4. "주소 생성" 버튼 클릭 → 즉시 API 호출
5. 콘솔에 결과 표시
6. 입금 주소 표시 (복사 가능)

**결과:** 입금 주소 확보, 외부에서 코인 수신 가능

### Journey 3: 출금 실행

**상황:** Upbit에서 Binance로 BTC 출금

**시스템 흐름:**
1. 거래소 탭에서 Upbit 선택
2. 입출금 패널 → 출금 탭
3. BTC 선택 → 네트워크 선택
4. 출금 주소 입력 (Binance 입금 주소)
5. 수량 입력
6. "출금" 버튼 클릭 → 즉시 API 호출
7. 콘솔에 출금 요청 결과 표시
8. 잔고 패널 갱신

**결과:** 출금 요청 완료, 거래소 간 자산 이동

### Journey 4: 에러 발생 - 복구 시나리오

**상황:** 주문 실행 중 API 에러 발생

**시스템 흐름:**
1. 매수 버튼 클릭 → API 호출
2. 거래소 API 에러 반환 (잔고 부족, rate limit, 네트워크 오류 등)
3. 콘솔에 에러 메시지 즉시 표시 (빨간색, 타임스탬프 포함)
4. UI에 에러 알림 표시
5. 사용자가 콘솔에서 원인 파악
6. 문제 해결 후 재시도

**결과:** 에러 원인 명확히 파악, 재시도 가능

### Journey Requirements Summary

| 여정 | 필요 기능 |
|------|----------|
| 매수/매도 | 거래소 탭, 잔고 조회, 오더북, 주문 입력, 콘솔 로그 |
| 입금 주소 생성 | 입출금 패널, 자산/네트워크 선택, 주소 생성 API |
| 출금 | 입출금 패널, 주소 입력, 수량 입력, 출금 API |
| 에러 처리 | 콘솔 로그 (실시간), UI 알림, 에러 분류 표시 |

---

## Domain-Specific Requirements

### 거래소 API 제약

| 거래소 | 인증 방식 | 주요 제약 |
|--------|----------|----------|
| Upbit | JWT + HMAC-SHA256 | 초당 10회, 출금 시 2FA 가능 |
| Bithumb | JWT + HMAC-SHA256 | 출금 시 인증 필요 가능 |
| Binance | HMAC-SHA256 | 1200 req/min, IP 화이트리스트 |
| Coinbase | ES256 (ECDSA) | CDP API 키 필요 |
| Bybit | HMAC-SHA256 | - |
| GateIO | HMAC-SHA512 | - |

### API 키 권한

- **필수 권한:** 출금 (Withdraw) 포함
- **권한 설정:** 각 거래소 API 명세에 따라 최소 필요 권한 설정

### 리스크 관리

| 기능 | 요구사항 |
|------|----------|
| 주문 확인 다이얼로그 | 매수/매도 전 확인 팝업 표시 |
| 출금 확인 다이얼로그 | 출금 전 주소/수량 확인 팝업 표시 |

### 출금 보안

- **2FA/이메일 인증:** 거래소 API 명세에 따름
  - API가 2FA 없이 출금 지원 시: WTS에서 직접 처리
  - API가 2FA 필요 시: 콘솔에 안내 메시지 표시, 거래소 웹에서 승인 필요

### 기술적 제약

| 항목 | 요구사항 |
|------|----------|
| Rate Limit 준수 | 거래소별 API 호출 제한 준수 |
| API 키 보안 | 기존 .env 파일 기반 관리 (현재 시스템과 동일) |
| 에러 처리 | API 에러 코드별 명확한 메시지 표시 |

---

## Desktop App Specific Requirements

### 플랫폼 지원

| 항목 | 요구사항 |
|------|----------|
| 크로스 플랫폼 | macOS, Windows, Linux (Tauri 기본 지원) |
| 프레임워크 | Tauri 2.0 (기존 시스템과 동일) |
| 프론트엔드 | React 18 + TypeScript |

### 창 관리

| 항목 | 요구사항 |
|------|----------|
| WTS 창 | 모니터링 앱과 별도의 독립 창 |
| 다중 창 | Tauri 다중 창 지원 활용 |
| 창 상태 | 크기/위치 기억 (선택적) |

### 배포 및 업데이트

| 항목 | 요구사항 |
|------|----------|
| 배포 방식 | 수동 빌드 후 배포 |
| 자동 업데이트 | 미지원 (추후 고려) |
| 설치 파일 | 플랫폼별 바이너리 (DMG, MSI, AppImage) |

### 오프라인 지원

| 항목 | 요구사항 |
|------|----------|
| 오프라인 모드 | 미지원 (거래소 API 필수) |
| 네트워크 필수 | 모든 기능이 거래소 API 연결 필요 |

### 기술 아키텍처

| 항목 | 요구사항 |
|------|----------|
| 백엔드 연동 | 기존 Rust 백엔드 활용 |
| IPC 통신 | Tauri Command/Event 시스템 |
| 상태 관리 | React Context 또는 Zustand |

---

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**MVP 접근 방식:** 문제 해결 MVP (Problem-Solving MVP)
- 핵심 문제: 거래소 웹 없이 거래 불가, 웹 장애 시 거래 중단
- MVP 목표: Upbit에서 거래소 웹 방문 없이 모든 거래 기능 수행

**리소스 요구사항:**
- 기존 Tauri/React/Rust 스택 활용
- 거래소 API 연동 경험 필요 (Upbit 우선)

### MVP Feature Set (Phase 1)

**Core User Journeys Supported:**
- Journey 1: 매수/매도 (시장가/지정가)
- Journey 2: 입금 주소 생성
- Journey 3: 출금 실행
- Journey 4: 에러 처리 및 복구

**Must-Have 패널 (6개):**

| 패널 | 필요성 |
|------|--------|
| 콘솔 | 에러/결과 확인 필수 |
| 거래소 탭 | 거래소 선택 필수 (MVP는 Upbit만) |
| 잔고 | 거래 전 잔고 확인 필수 |
| 오더북 | 호가 확인 후 주문 필수 |
| 매수/매도 | 핵심 기능 |
| 입출금 | 핵심 기능 |

**Phase 2로 이동한 패널 (2개):**

| 패널 | 이유 |
|------|------|
| 환율 계산기/메모 | 편의 기능, 거래에 필수 아님 |
| 거래소 링크 | 편의 기능, 브라우저 북마크로 대체 가능 |

**Must-Have Capabilities:**
- Upbit API 연동 (잔고, 오더북, 주문, 입출금)
- WebSocket 오더북 실시간 스트리밍
- 주문/출금 확인 다이얼로그
- 콘솔 실시간 로그
- Tauri 별도 창 생성

### Post-MVP Features

**Phase 2 (Growth):**
- 추가 거래소: Bithumb, Binance, Bybit, GateIO, Coinbase
- 환율 계산기/메모 패널
- 거래소 간편 링크 패널
- Stop-Limit, OCO 주문 유형

**Phase 3 (Expansion):**
- 차익거래 시나리오 자동 실행
- 멀티 거래소 동시 주문
- 거래 히스토리 분석

### Risk Mitigation Strategy

**Technical Risks:**

| 리스크 | 완화 방안 |
|--------|----------|
| API 연동 복잡성 | Upbit 단일 거래소로 MVP 범위 한정 |
| 거래소별 API 차이 | Phase 2에서 점진적 확장 |
| 실시간 오더북 | 기존 WebSocket 인프라 활용 |

**Market Risks:**

| 리스크 | 완화 방안 |
|--------|----------|
| 거래소 API 정책 변경 | API 버전 관리, 에러 핸들링 강화 |

**Resource Risks:**

| 리스크 | 완화 방안 |
|--------|----------|
| 리소스 부족 시 | 패널 수 6개로 축소 완료, 더 축소 시 입출금 패널 분리 고려 |

---

## Functional Requirements

### 거래소 관리

- FR1: 사용자는 거래소 탭에서 거래소를 선택할 수 있다
- FR2: 사용자는 선택한 거래소의 연결 상태를 확인할 수 있다
- FR3: 시스템은 거래소 API 장애 시 상태를 표시한다

### 잔고 조회

- FR4: 사용자는 선택한 거래소의 자산별 잔고를 조회할 수 있다
- FR5: 사용자는 잔고 정보를 갱신할 수 있다
- FR6: 시스템은 거래 완료 후 잔고를 자동 갱신한다

### 오더북

- FR7: 사용자는 선택한 마켓의 실시간 호가창을 볼 수 있다
- FR8: 사용자는 거래할 마켓(예: BTC/KRW)을 선택할 수 있다
- FR9: 시스템은 WebSocket을 통해 오더북을 실시간 스트리밍한다

### 주문

- FR10: 사용자는 시장가 매수 주문을 실행할 수 있다
- FR11: 사용자는 시장가 매도 주문을 실행할 수 있다
- FR12: 사용자는 지정가 매수 주문을 실행할 수 있다
- FR13: 사용자는 지정가 매도 주문을 실행할 수 있다
- FR14: 사용자는 주문 수량과 가격을 입력할 수 있다
- FR15: 시스템은 주문 실행 전 확인 다이얼로그를 표시한다
- FR16: 시스템은 주문 결과를 콘솔에 즉시 표시한다

### 입금

- FR17: 사용자는 입금할 자산을 선택할 수 있다
- FR18: 사용자는 입금 네트워크를 선택할 수 있다
- FR19: 사용자는 입금 주소를 생성할 수 있다
- FR20: 사용자는 생성된 입금 주소를 복사할 수 있다

### 출금

- FR21: 사용자는 출금할 자산을 선택할 수 있다
- FR22: 사용자는 출금 네트워크를 선택할 수 있다
- FR23: 사용자는 출금 주소를 입력할 수 있다
- FR24: 사용자는 출금 수량을 입력할 수 있다
- FR25: 시스템은 출금 실행 전 확인 다이얼로그를 표시한다
- FR26: 사용자는 출금을 실행할 수 있다
- FR27: 시스템은 2FA 필요 시 안내 메시지를 표시한다

### 콘솔 로그

- FR28: 시스템은 모든 API 요청/응답을 콘솔에 기록한다
- FR29: 시스템은 에러 발생 시 타임스탬프와 함께 빨간색으로 표시한다
- FR30: 시스템은 성공 메시지를 구분하여 표시한다
- FR31: 사용자는 콘솔 로그를 스크롤하여 이전 기록을 볼 수 있다

### 창 관리

- FR32: 사용자는 모니터링 앱에서 WTS 창을 열 수 있다
- FR33: WTS 창은 모니터링 앱과 독립적으로 동작한다

### 에러 처리

- FR34: 시스템은 API 에러 코드별 명확한 메시지를 표시한다
- FR35: 시스템은 Rate Limit 초과 시 사용자에게 알린다
- FR36: 시스템은 네트워크 오류 시 사용자에게 알린다

---

## Non-Functional Requirements

### Performance

| 항목 | 요구사항 |
|------|----------|
| 주문 실행 | 버튼 클릭 → API 호출 즉시 (내부 지연 없음) |
| 오더북 갱신 | WebSocket 메시지 수신 → UI 갱신 100ms 이내 |
| 잔고 갱신 | 거래 완료 후 1초 이내 갱신 |
| 콘솔 로그 | 이벤트 발생 → 표시 100ms 이내 |
| UI 반응성 | 사용자 인터랙션 → 피드백 200ms 이내 |
| 배치 처리 | 금지 (모든 요청 개별 즉시 처리) |

### Security

| 항목 | 요구사항 |
|------|----------|
| API 키 저장 | .env 파일 기반 (기존 시스템 동일) |
| API 키 전송 | HTTPS 통신만 사용 |
| 출금 확인 | 출금 전 주소/수량 확인 다이얼로그 필수 |
| 주문 확인 | 주문 전 확인 다이얼로그 필수 |
| 메모리 보안 | API 키 평문 로깅 금지 |

### Integration

| 항목 | 요구사항 |
|------|----------|
| 거래소 API | REST + WebSocket 지원 |
| 인증 방식 | 거래소별 명세 준수 (JWT, HMAC-SHA256, ES256 등) |
| Rate Limit | 거래소별 호출 제한 준수 |
| 에러 처리 | 거래소별 에러 코드 파싱 및 표시 |
| 재연결 | WebSocket 연결 끊김 시 자동 재연결 |

### Reliability

| 항목 | 요구사항 |
|------|----------|
| 안정성 | 장시간 무중단 운영 목표 |
| 거래소 장애 대응 | API 장애 감지 및 상태 표시 |
| 에러 복구 | 일시적 네트워크 오류 시 재시도 |
| 데이터 정합성 | 주문/출금 결과 정확히 반영 |

