use axum::{response::Html, response::IntoResponse, routing::get, Router};
use serde_json::Value;
use std::env;
use std::time::Duration;
use tokio::net::TcpListener;

/// HTTP-Client mit Timeout, damit Leapcell nicht "failed to respond" meldet.
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

const BENZINPREIS_AKTUELL_URL: &str =
    "https://benzinpreis-aktuell.de/api.v2.php?data=nationwide&apikey";
const TANKERKOENIG_BASE: &str = "https://creativecommons.tankerkoenig.de/json";

/// Einheitliches Ergebnis für die Anzeige (von beliebiger API).
struct PriceData {
    station_name: String,
    e5: f64,
    e10: f64,
    diesel: f64,
    updated: String,
}

/// Kostenlose API: Bundesweite Durchschnittspreise (ohne API-Key).
/// Antwortformat: {"date":"2026-02-20 18:50:01","super":"1.813","e10":"1.756","diesel":"1.714"}
async fn fetch_benzinpreis_aktuell() -> Option<PriceData> {
    let resp = http_client().get(BENZINPREIS_AKTUELL_URL).send().await.ok()?;
    let json: Value = resp.json().await.ok()?;
    let e5 = json
        .get("super")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let e10 = json
        .get("e10")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let diesel = json
        .get("diesel")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let updated = json
        .get("date")
        .and_then(|v| v.as_str())
        .unwrap_or("–")
        .to_string();
    Some(PriceData {
        station_name: "Bundesweite Durchschnittspreise".to_string(),
        e5,
        e10,
        diesel,
        updated,
    })
}

/// Lädt Name der Tankstelle (detail.php).
async fn fetch_station_name(api_key: &str, station_id: &str) -> Option<String> {
    let url = format!(
        "{}/detail.php?id={}&apikey={}",
        TANKERKOENIG_BASE, station_id, api_key
    );
    let resp = http_client().get(&url).send().await.ok()?;
    let json: Value = resp.json().await.ok()?;
    let station = json.get("station")?;
    let name = station.get("name")?.as_str()?;
    let brand = station.get("brand").and_then(|b| b.as_str()).unwrap_or("");
    let label = if brand.is_empty() {
        name.to_string()
    } else {
        format!("{} {}", brand, name)
    };
    Some(label)
}

/// Lädt aktuelle Preise (prices.php): (e5, e10, diesel).
async fn fetch_tankerkoenig_prices(api_key: &str, station_id: &str) -> Option<(f64, f64, f64)> {
    let url = format!(
        "{}/prices.php?ids={}&apikey={}",
        TANKERKOENIG_BASE, station_id, api_key
    );
    let resp = http_client().get(&url).send().await.ok()?;
    let json: Value = resp.json().await.ok()?;
    if !json.get("ok")?.as_bool()? {
        return None;
    }
    let prices = json.get("prices")?.get(station_id)?;
    let e5 = prices.get("e5").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let e10 = prices.get("e10").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let diesel = prices.get("diesel").and_then(|v| v.as_f64()).unwrap_or(0.0);
    Some((e5, e10, diesel))
}

/// Tankerkönig: Preise + Name einer konkreten Tankstelle (später nutzbar).
async fn fetch_tankerkoenig(api_key: &str, station_id: &str) -> Option<PriceData> {
    let name = fetch_station_name(api_key, station_id)
        .await
        .unwrap_or_else(|| "Meine Tankstelle".to_string());
    let (e5, e10, diesel) = fetch_tankerkoenig_prices(api_key, station_id).await?;
    Some(PriceData {
        station_name: name,
        e5,
        e10,
        diesel,
        updated: "Live (Tankerkönig)".to_string(),
    })
}

/// Health-Check für Leapcell: so prüft die Plattform, ob der Dienst antwortet.
async fn health() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "ok")
}

async fn dashboard() -> Html<String> {
    let api_key = env::var("TANKERKOENIG_API_KEY").unwrap_or_default();
    let station_id = env::var("TANKERKOENIG_STATION_ID").unwrap_or_default();

    // Wenn Tankerkönig konfiguriert ist: konkrete Tankstelle; sonst: kostenlose Bundesdurchschnitte
    let default_fallback = PriceData {
        station_name: "Bundesweite Durchschnittspreise".to_string(),
        e5: 0.0,
        e10: 0.0,
        diesel: 0.0,
        updated: "–".to_string(),
    };
    let data = if !api_key.is_empty() && !station_id.is_empty() {
        if let Some(d) = fetch_tankerkoenig(&api_key, &station_id).await {
            d
        } else {
            fetch_benzinpreis_aktuell()
                .await
                .unwrap_or(default_fallback)
        }
    } else {
        fetch_benzinpreis_aktuell()
            .await
            .unwrap_or(default_fallback)
    };

    let config_hint = if api_key.is_empty() || station_id.is_empty() {
        r#"<div class="hint">Optional: TANKERKOENIG_API_KEY + TANKERKOENIG_STATION_ID setzen für Preise deiner Tankstelle.</div>"#.to_string()
    } else {
        String::new()
    };

    Html(format!(
        r#"
<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="refresh" content="60">
<title>Kraftstoff Dashboard</title>
<style>
    body {{
        margin: 0;
        font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
        background: radial-gradient(circle at top, #ffffff 0%, #f2f2f7 40%, #d1d1d6 100%);
        display: flex;
        justify-content: center;
        align-items: center;
        min-height: 100vh;
        padding: 24px;
        color: #111827;
    }}
    .container {{
        width: 380px;
        max-width: 100%;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }}
    .card {{
        background: rgba(255, 255, 255, 0.55);
        backdrop-filter: blur(30px) saturate(160%);
        -webkit-backdrop-filter: blur(30px) saturate(160%);
        border-radius: 28px;
        padding: 20px 22px;
        box-shadow: 0 22px 50px rgba(0,0,0,0.28);
        border: 1px solid rgba(255, 255, 255, 0.9);
        text-align: center;
    }}
    .card-header {{
        padding-bottom: 4px;
    }}
    .title-main {{
        font-size: 26px;
        font-weight: 900;
        letter-spacing: 0.03em;
        margin: 0;
        color: #000000;
    }}
    .title-sub {{
        font-size: 16px;
        font-weight: 700;
        margin-top: 6px;
        color: #1c1c1e;
    }}
    .fuel-row {{
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        margin: 8px 0;
    }}
    .fuel-label {{
        font-size: 16px;
        font-weight: 700;
        color: #000000;
    }}
    .fuel-label.e10 {{
        color: #ff3b30;
    }}
    .price {{
        font-size: 34px;
        font-weight: 900;
        color: #000000;
    }}
    .price.e10 {{
        color: #ff3b30;
    }}
    .updated {{
        font-size: 13px;
        font-weight: 600;
        color: #1c1c1e;
        margin-top: 4px;
    }}
    .footer {{
        font-size: 12px;
        color: #8e8e93;
        font-weight: 600;
        text-align: center;
        margin-top: 2px;
    }}
    .hint {{
        font-size: 12px;
        color: #888;
        margin-top: 8px;
    }}
</style>
</head>
<body>
    <div class="container">
        <div class="card">
            <div class="card-header">
                <h1 class="title-main">Kraftstoffpreis aktuell</h1>
                <div class="title-sub">Tankstelle Weiterstadt</div>
            </div>

            <div class="fuel-row">
                <div class="fuel-label">Super E5</div>
                <div class="price">{}</div>
            </div>

            <div class="fuel-row">
                <div class="fuel-label e10">Super E10</div>
                <div class="price e10">{}</div>
            </div>

            <div class="fuel-row">
                <div class="fuel-label">Diesel</div>
                <div class="price">{}</div>
            </div>
        </div>
        <div class="card">
            <div class="updated">Zuletzt aktualisiert: {}</div>
            <div class="footer">developed by Lionel</div>
        </div>
    </div>
</body>
</html>
"#,
        if data.e5 > 0.0 {
            format!("{:.2} €", data.e5)
        } else {
            "– €".to_string()
        },
        if data.e10 > 0.0 {
            format!("{:.2} €", data.e10)
        } else {
            "– €".to_string()
        },
        if data.diesel > 0.0 {
            format!("{:.2} €", data.diesel)
        } else {
            "– €".to_string()
        },
        data.updated,
        // config_hint entfernt, da Credits-Blase nur Zeitstempel + developed by enthält
    ))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(dashboard))
        .route("/health", get(health))
        .route("/kaithhealth", get(health)); // von Leapcell beim Start abgefragt

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&bind).await.expect("Port binden");
    println!("Server läuft auf http://{}", bind);

    axum::serve(listener, app).await.expect("Server starten");
}
