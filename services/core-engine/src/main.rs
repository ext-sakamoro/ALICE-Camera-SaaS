use axum::{extract::State, response::Json, routing::{get, post}, Router};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

struct AppState { start_time: Instant, stats: Mutex<Stats> }
struct Stats { total_process_ops: u64, total_pipeline_runs: u64, total_calibrations: u64, total_pixels_processed: u64 }

#[derive(Serialize)]
struct Health { status: String, version: String, uptime_secs: u64, total_ops: u64 }

#[derive(Deserialize)]
struct ProcessRequest { raw_b64: String, bayer_pattern: Option<String>, preset: Option<String>, output_format: Option<String>, width: Option<u32>, height: Option<u32> }
#[derive(Serialize)]
struct IspMetrics { wb_gains: [f32; 4], exposure_ev: f32, noise_reduction_strength: f32, sharpness_score: f32 }
#[derive(Serialize)]
struct ProcessResponse { job_id: String, image_b64: String, format: String, width: u32, height: u32, isp_metrics: IspMetrics, processing_ms: u128 }

#[derive(Deserialize)]
struct StageConfig { name: String, enabled: bool, params: Option<serde_json::Value> }
#[derive(Deserialize)]
struct PipelineRequest { raw_b64: String, stages: Vec<StageConfig>, output_format: Option<String> }
#[derive(Serialize)]
struct StageResult { name: String, duration_us: u64, applied: bool }
#[derive(Serialize)]
struct PipelineResponse { job_id: String, image_b64: String, format: String, stages: Vec<StageResult>, total_processing_ms: u128 }

#[derive(Deserialize)]
struct CalibrateRequest { images_b64: Vec<String>, pattern: Option<String>, pattern_size: Option<[u32; 2]> }
#[derive(Serialize)]
struct IntrinsicMatrix { fx: f64, fy: f64, cx: f64, cy: f64 }
#[derive(Serialize)]
struct CalibrateResponse { job_id: String, intrinsic: IntrinsicMatrix, distortion_coeffs: Vec<f64>, reprojection_error_px: f64, image_count: u32, processing_ms: u128 }

#[derive(Serialize)]
struct PresetInfo { id: String, name: String, description: String, target_colorspace: String, tags: Vec<String> }

#[derive(Serialize)]
struct StatsResponse { total_process_ops: u64, total_pipeline_runs: u64, total_calibrations: u64, total_pixels_processed: u64 }

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "camera_engine=info".into())).init();
    let state = Arc::new(AppState { start_time: Instant::now(), stats: Mutex::new(Stats { total_process_ops: 0, total_pipeline_runs: 0, total_calibrations: 0, total_pixels_processed: 0 }) });
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/camera/process", post(process))
        .route("/api/v1/camera/pipeline", post(pipeline))
        .route("/api/v1/camera/calibrate", post(calibrate))
        .route("/api/v1/camera/presets", get(presets))
        .route("/api/v1/camera/stats", get(stats))
        .layer(cors).layer(TraceLayer::new_for_http()).with_state(state);
    let addr = std::env::var("CAMERA_ADDR").unwrap_or_else(|_| "0.0.0.0:8116".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Camera Engine on {addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn health(State(s): State<Arc<AppState>>) -> Json<Health> {
    let st = s.stats.lock().unwrap();
    Json(Health { status: "ok".into(), version: env!("CARGO_PKG_VERSION").into(), uptime_secs: s.start_time.elapsed().as_secs(), total_ops: st.total_process_ops + st.total_pipeline_runs + st.total_calibrations })
}

async fn process(State(s): State<Arc<AppState>>, Json(req): Json<ProcessRequest>) -> Json<ProcessResponse> {
    let t = Instant::now();
    let w = req.width.unwrap_or(4000);
    let h = req.height.unwrap_or(3000);
    let format = req.output_format.unwrap_or_else(|| "jpeg".into());
    let pixels = (w as u64) * (h as u64);
    { let mut st = s.stats.lock().unwrap(); st.total_process_ops += 1; st.total_pixels_processed += pixels; }
    Json(ProcessResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        image_b64: "/9j/4AAQSkZJRgAB".into(),
        format,
        width: w,
        height: h,
        isp_metrics: IspMetrics { wb_gains: [1.82, 1.0, 1.0, 1.54], exposure_ev: 0.33, noise_reduction_strength: 0.6, sharpness_score: 87.4 },
        processing_ms: t.elapsed().as_millis(),
    })
}

async fn pipeline(State(s): State<Arc<AppState>>, Json(req): Json<PipelineRequest>) -> Json<PipelineResponse> {
    let t = Instant::now();
    let format = req.output_format.unwrap_or_else(|| "jpeg".into());
    let stage_results: Vec<StageResult> = req.stages.iter().map(|st| StageResult { name: st.name.clone(), duration_us: 800, applied: st.enabled }).collect();
    { let mut st = s.stats.lock().unwrap(); st.total_pipeline_runs += 1; st.total_pixels_processed += (req.raw_b64.len() / 3) as u64; }
    Json(PipelineResponse { job_id: uuid::Uuid::new_v4().to_string(), image_b64: "/9j/4AAQSkZJRgAB".into(), format, stages: stage_results, total_processing_ms: t.elapsed().as_millis() })
}

async fn calibrate(State(s): State<Arc<AppState>>, Json(req): Json<CalibrateRequest>) -> Json<CalibrateResponse> {
    let t = Instant::now();
    let count = req.images_b64.len() as u32;
    { let mut st = s.stats.lock().unwrap(); st.total_calibrations += 1; }
    Json(CalibrateResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        intrinsic: IntrinsicMatrix { fx: 3500.0, fy: 3500.2, cx: 2000.1, cy: 1500.3 },
        distortion_coeffs: vec![-0.12345, 0.08921, 0.00012, -0.00034, -0.04567],
        reprojection_error_px: 0.38,
        image_count: count,
        processing_ms: t.elapsed().as_millis(),
    })
}

async fn presets() -> Json<Vec<PresetInfo>> {
    Json(vec![
        PresetInfo { id: "standard-srgb".into(), name: "Standard sRGB".into(), description: "Default sRGB output for web and display".into(), target_colorspace: "sRGB".into(), tags: vec!["default".into(), "web".into()] },
        PresetInfo { id: "cinema-d65".into(), name: "Cinema D65".into(), description: "Cinema-grade color grading with D65 white point".into(), target_colorspace: "DCI-P3".into(), tags: vec!["cinema".into(), "hdr".into()] },
        PresetInfo { id: "log-v3".into(), name: "ALICE Log v3".into(), description: "Flat log profile for maximum dynamic range preservation".into(), target_colorspace: "ALICE-Log-v3".into(), tags: vec!["log".into(), "grading".into()] },
        PresetInfo { id: "night-mode".into(), name: "Night Mode".into(), description: "High ISO noise reduction with shadow lift".into(), target_colorspace: "sRGB".into(), tags: vec!["night".into(), "denoise".into()] },
    ])
}

async fn stats(State(s): State<Arc<AppState>>) -> Json<StatsResponse> {
    let st = s.stats.lock().unwrap();
    Json(StatsResponse { total_process_ops: st.total_process_ops, total_pipeline_runs: st.total_pipeline_runs, total_calibrations: st.total_calibrations, total_pixels_processed: st.total_pixels_processed })
}
