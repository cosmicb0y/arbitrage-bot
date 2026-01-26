//! Upbit myOrder WebSocket Client
//!
//! Rust 백엔드에서 Upbit private WebSocket 연결을 관리합니다.
//! 브라우저 WebSocket API는 커스텀 HTTP 헤더를 지원하지 않아
//! Upbit 인증이 불가능하므로, Rust에서 연결 후 Tauri 이벤트로 전달합니다.

use super::auth::generate_jwt_token;
use futures_util::{SinkExt, StreamExt};
use native_tls::TlsConnector;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{client::IntoClientRequest, Message},
    Connector,
};
use tracing::{error, info, warn};

const UPBIT_WS_URL: &str = "wss://api.upbit.com/websocket/v1/private";

/// 연결 진행 중 상태 (중복 연결 방지 - React StrictMode 대응)
static IS_CONNECTING: AtomicBool = AtomicBool::new(false);

/// WebSocket 연결 핸들을 저장하는 전역 상태
static WS_HANDLE: OnceLock<Mutex<Option<tokio::task::JoinHandle<()>>>> = OnceLock::new();

fn ws_handle() -> &'static Mutex<Option<tokio::task::JoinHandle<()>>> {
    WS_HANDLE.get_or_init(|| Mutex::new(None))
}

/// 환경 변수에서 API 키를 로드합니다.
fn load_api_keys() -> Result<(String, String), String> {
    let _ = crate::credentials::load_credentials();

    let access_key =
        std::env::var("UPBIT_ACCESS_KEY").map_err(|_| "UPBIT_ACCESS_KEY 환경 변수가 없습니다")?;
    let secret_key =
        std::env::var("UPBIT_SECRET_KEY").map_err(|_| "UPBIT_SECRET_KEY 환경 변수가 없습니다")?;

    if access_key.is_empty() || secret_key.is_empty() {
        return Err("Upbit API 키가 설정되지 않았습니다".to_string());
    }

    Ok((access_key, secret_key))
}

/// 내부용 연결 중지 (IS_CONNECTING 플래그 변경 안 함)
async fn stop_myorder_ws_internal() {
    let mut handle_guard = ws_handle().lock().await;
    if let Some(handle) = handle_guard.take() {
        handle.abort();
    }
}

/// Upbit myOrder WebSocket 연결을 시작합니다.
///
/// # Arguments
/// * `app` - Tauri AppHandle (이벤트 emit용)
///
/// # Returns
/// * `Ok(())` - 연결 성공
/// * `Err(String)` - 연결 실패 시 에러 메시지
pub async fn start_myorder_ws(app: AppHandle) -> Result<(), String> {
    // 이미 연결 중이면 중복 연결 방지 (React StrictMode 대응)
    if IS_CONNECTING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }

    // 기존 연결이 있고 아직 실행 중이면 skip
    {
        let handle_guard = ws_handle().lock().await;
        if let Some(handle) = handle_guard.as_ref() {
            if !handle.is_finished() {
                IS_CONNECTING.store(false, Ordering::SeqCst);
                return Ok(());
            }
        }
    }

    // 기존 연결 정리 (완료된 핸들 제거)
    stop_myorder_ws_internal().await;

    // 연결 실패 시 플래그 복원을 위한 매크로
    macro_rules! fail {
        ($e:expr) => {{
            IS_CONNECTING.store(false, Ordering::SeqCst);
            return Err($e);
        }};
    }

    let (access_key, secret_key) = match load_api_keys() {
        Ok(keys) => keys,
        Err(e) => fail!(e),
    };

    let token = match generate_jwt_token(&access_key, &secret_key) {
        Ok(t) => t,
        Err(e) => fail!(e),
    };

    info!("myOrder WebSocket 연결 시도...");

    // TLS Connector 생성 (TLS 1.2 이상 필수)
    let tls_connector = match TlsConnector::builder()
        .min_protocol_version(Some(native_tls::Protocol::Tlsv12))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[myOrder WS] TLS 설정 실패: {:?}", e);
            fail!(format!("TLS 설정 실패: {:?}", e));
        }
    };

    let connector = Connector::NativeTls(tls_connector);

    // HTTP 요청 생성
    let mut request = match UPBIT_WS_URL.into_client_request() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[myOrder WS] 요청 생성 실패: {:?}", e);
            fail!(format!("WebSocket 요청 생성 실패: {:?}", e));
        }
    };

    // Authorization 헤더 추가
    let auth_header = match format!("Bearer {}", token).parse() {
        Ok(h) => h,
        Err(e) => fail!(format!("Authorization 헤더 파싱 실패: {}", e)),
    };
    request.headers_mut().insert("Authorization", auth_header);

    // TLS 설정과 함께 연결
    let (ws_stream, _response) =
        match connect_async_tls_with_config(request, None, false, Some(connector)).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[myOrder WS] 연결 실패: {:?}", e);
                error!("WebSocket 연결 실패 상세: {:?}", e);
                fail!(format!("WebSocket 연결 실패: {:?}", e));
            }
        };

    info!("myOrder WebSocket 연결 성공");

    let (mut write, mut read) = ws_stream.split();

    // 구독 메시지 전송
    let subscribe_msg = serde_json::json!([
        {"ticket": format!("myorder-{}", uuid::Uuid::new_v4())},
        {"type": "myOrder"}
    ]);

    if let Err(e) = write
        .send(Message::Text(subscribe_msg.to_string().into()))
        .await
    {
        println!("[myOrder WS] 구독 메시지 전송 실패: {:?}", e);
        fail!(format!("구독 메시지 전송 실패: {:?}", e));
    }

    info!("myOrder 구독 메시지 전송 완료");

    // 연결 성공 이벤트 emit
    let _ = app.emit("wts:myorder:status", "connected");

    // 메시지 수신 루프를 별도 태스크로 실행
    let app_clone = app.clone();
    let handle = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = app_clone.emit("wts:myorder:message", text.to_string()) {
                        error!("myOrder 이벤트 emit 실패: {}", e);
                    }
                }
                Ok(Message::Binary(data)) => {
                    if let Ok(text) = String::from_utf8(data.to_vec()) {
                        if let Err(e) = app_clone.emit("wts:myorder:message", text) {
                            error!("myOrder 이벤트 emit 실패: {}", e);
                        }
                    }
                }
                Ok(Message::Close(frame)) => {
                    warn!("myOrder WebSocket 종료: {:?}", frame);
                    let _ = app_clone.emit("wts:myorder:status", "disconnected");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    // Ping에 대한 Pong 응답은 tungstenite가 자동 처리
                }
                Ok(Message::Pong(_)) => {
                    // Pong 응답 무시
                }
                Ok(Message::Frame(_)) => {
                    // Raw frame 무시
                }
                Err(e) => {
                    error!("myOrder WebSocket 에러: {}", e);
                    let _ = app_clone.emit("wts:myorder:status", "error");
                    let _ = app_clone.emit("wts:myorder:error", e.to_string());
                    break;
                }
            }
        }

        // 연결 종료 이벤트
        let _ = app_clone.emit("wts:myorder:status", "disconnected");
        // 연결 종료 시 플래그 리셋
        IS_CONNECTING.store(false, Ordering::SeqCst);
    });

    // 핸들 저장
    *ws_handle().lock().await = Some(handle);

    // 연결 성공, 플래그는 task 종료 시 리셋됨
    Ok(())
}

/// Upbit myOrder WebSocket 연결을 중지합니다.
///
/// # Returns
/// * `Ok(())` - 중지 성공 또는 이미 중지된 상태
pub async fn stop_myorder_ws() -> Result<(), String> {
    // 연결 플래그 리셋
    IS_CONNECTING.store(false, Ordering::SeqCst);

    let mut handle_guard = ws_handle().lock().await;
    if let Some(handle) = handle_guard.take() {
        info!("myOrder WebSocket 연결 중지 중...");
        handle.abort();
        info!("myOrder WebSocket 연결 중지됨");
    }
    Ok(())
}
