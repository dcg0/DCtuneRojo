//! Background metrics emission task.

use crate::state::AppState;
use tauri::{Emitter, Manager};

/// Start periodic connection metrics emission task (1s interval)
pub(crate) async fn start_metrics_task(app: tauri::AppHandle, state: tauri::State<'_, AppState>) {
    let mut guard = state.metrics_task.lock().await;
    // If already running, do nothing
    if guard.is_some() {
        return;
    }

    let app_handle = app.clone();

    let handle = tokio::spawn(async move {
        use tokio::time::{sleep, Duration};
        // Obtain AppState inside the spawned task via AppHandle to ensure 'static lifetime
        let state = app_handle.state::<AppState>();
        let mut prev_tx: u64 = 0;
        let mut prev_rx: u64 = 0;
        let mut prev_tx_pkts: u64 = 0;
        let mut prev_rx_pkts: u64 = 0;
        let mut prev_ts = std::time::Instant::now();

        loop {
            sleep(Duration::from_millis(1000)).await;

            // Sample connection counters
            let (tx, rx, tx_pkts, rx_pkts, connected) = {
                let conn_guard = state.connection.lock().await;
                if let Some(conn) = conn_guard.as_ref() {
                    // get counters
                    let (tx_b, rx_b, tx_p, rx_p) = conn.get_counters();
                    (tx_b, rx_b, tx_p, rx_p, true)
                } else {
                    (0u64, 0u64, 0u64, 0u64, false)
                }
            };

            let now = std::time::Instant::now();
            let dt = now.duration_since(prev_ts).as_secs_f64();
            prev_ts = now;

            if connected {
                // Deltas
                let dtx = tx.saturating_sub(prev_tx) as f64;
                let drx = rx.saturating_sub(prev_rx) as f64;
                let dtxp = tx_pkts.saturating_sub(prev_tx_pkts) as f64;
                let drxp = rx_pkts.saturating_sub(prev_rx_pkts) as f64;

                prev_tx = tx;
                prev_rx = rx;
                prev_tx_pkts = tx_pkts;
                prev_rx_pkts = rx_pkts;

                // Rates
                let tx_bps = if dt > 0.0 { dtx / dt } else { 0.0 };
                let rx_bps = if dt > 0.0 { drx / dt } else { 0.0 };
                let tx_pkts_s = if dt > 0.0 { dtxp / dt } else { 0.0 };
                let rx_pkts_s = if dt > 0.0 { drxp / dt } else { 0.0 };

                // Include stream stats snapshot in metrics payload
                let stream_snapshot = {
                    match state.stream_stats.try_lock() {
                        Ok(s) => Some(s.clone()),
                        Err(_) => None,
                    }
                };

                let mut payload = serde_json::json!({
                    "tx_bps": tx_bps,
                    "rx_bps": rx_bps,
                    "tx_pkts_s": tx_pkts_s,
                    "rx_pkts_s": rx_pkts_s,
                    "tx_total": tx,
                    "rx_total": rx,
                    "timestamp_ms": chrono::Utc::now().timestamp_millis()
                });
                if let Some(ss) = stream_snapshot {
                    if let Ok(ss_val) = serde_json::to_value(&ss) {
                        payload
                            .as_object_mut()
                            .unwrap()
                            .insert("stream".to_string(), ss_val);
                    }
                }

                let _ = app_handle.emit("connection:metrics", payload);
            } else {
                // Not connected - emit zero metrics to update UI
                let payload = serde_json::json!({
                    "tx_bps": 0.0,
                    "rx_bps": 0.0,
                    "tx_pkts_s": 0.0,
                    "rx_pkts_s": 0.0,
                    "tx_total": tx,
                    "rx_total": rx,
                    "timestamp_ms": chrono::Utc::now().timestamp_millis()
                });
                let _ = app_handle.emit("connection:metrics", payload);
            }
        }
    });

    *guard = Some(handle);
}

/// Stop metrics task if running
pub(crate) async fn stop_metrics_task(state: tauri::State<'_, AppState>) {
    let mut guard = state.metrics_task.lock().await;
    if let Some(handle) = guard.take() {
        handle.abort();
    }
}
