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

const TANKERKOENIG_BASE: &str = "https://creativecommons.tankerkoenig.de/json";
// Weiterstadt, Hessen Koordinaten: 49.91°N, 8.58°E
const WEITERSTADT_LAT: &str = "49.91";
const WEITERSTADT_LNG: &str = "8.58";
const SEARCH_RADIUS: &str = "5"; // 5 km Radius

/// Einheitliches Ergebnis für die Anzeige (von beliebiger API).
struct PriceData {
    station_name: String,
    e5: f64,
    e10: f64,
    diesel: f64,
    updated: String,
}

/// Lädt Tankstellen in Weiterstadt über list.php API.
/// Antwortformat: {"ok":true,"stations":[{"id":"...","name":"...","brand":"...","e5":1.779,"e10":1.719,"diesel":1.679,...},...]}
async fn fetch_weiterstadt_stations(api_key: &str) -> Option<PriceData> {
    let url = format!(
        "{}/list.php?lat={}&lng={}&rad={}&sort=dist&type=all&apikey={}",
        TANKERKOENIG_BASE, WEITERSTADT_LAT, WEITERSTADT_LNG, SEARCH_RADIUS, api_key
    );
    let resp = http_client().get(&url).send().await.ok()?;
    let json: Value = resp.json().await.ok()?;
    
    if !json.get("ok")?.as_bool()? {
        return None;
    }
    
    // Nimm die erste (nächste) Tankstelle aus der Liste
    let stations = json.get("stations")?.as_array()?;
    let station = stations.first()?;
    
    let name = station.get("name")?.as_str()?.to_string();
    let brand = station.get("brand").and_then(|b| b.as_str()).unwrap_or("");
    let station_name = if brand.is_empty() {
        name
    } else {
        format!("{} {}", brand, name)
    };
    
    let e5 = station.get("e5").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let e10 = station.get("e10").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let diesel = station.get("diesel").and_then(|v| v.as_f64()).unwrap_or(0.0);
    
    Some(PriceData {
        station_name,
        e5,
        e10,
        diesel,
        updated: "Live".to_string(),
    })
}

/// Health-Check für Leapcell: so prüft die Plattform, ob der Dienst antwortet.
async fn health() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "ok")
}

async fn dashboard() -> Html<String> {
    let api_key = env::var("TANKERKOENIG_API_KEY")
        .unwrap_or_else(|_| "4f98d489-ed79-46e9-93a9-f0e79ab92add".to_string()); // Fallback API-Key

    let default_fallback = PriceData {
        station_name: "Tankstelle Weiterstadt".to_string(),
        e5: 0.0,
        e10: 0.0,
        diesel: 0.0,
        updated: "–".to_string(),
    };
    
    let data = fetch_weiterstadt_stations(&api_key)
        .await
        .unwrap_or(default_fallback);

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
        background: radial-gradient(ellipse at 50% 20%, #faf5ff 0%, #f0e8ff 50%, #e6d5ff 100%);
        display: flex;
        justify-content: center;
        align-items: center;
        min-height: 100vh;
        padding: 28px;
        color: #111827;
    }}
    .container {{
        width: 420px;
        max-width: 100%;
        display: flex;
        flex-direction: column;
        gap: 20px;
    }}
    .card {{
        background: rgba(255, 255, 255, 0.38);
        backdrop-filter: blur(42px) saturate(180%);
        -webkit-backdrop-filter: blur(42px) saturate(180%);
        border-radius: 32px;
        padding: 28px 30px;
        box-shadow: 0 28px 60px rgba(0,0,0,0.35),
                    inset 0 1px 0 rgba(255,255,255,0.85);
        border: 1px solid rgba(255, 255, 255, 0.75);
        text-align: center;
    }}
    .card-header {{
        padding-bottom: 8px;
    }}
    .title-main {{
        font-size: 32px;
        font-weight: 900;
        letter-spacing: 0.03em;
        margin: 0;
        color: #000000;
    }}
    .title-sub {{
        font-size: 20px;
        font-weight: 700;
        margin-top: 10px;
        color: #1c1c1e;
    }}
    .fuel-row {{
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        margin: 14px 0;
    }}
    .fuel-label {{
        font-size: 20px;
        font-weight: 700;
        color: #000000;
    }}
    .fuel-label.e10 {{
        color: #ff3b30;
    }}
    .price {{
        font-size: 44px;
        font-weight: 900;
        color: #000000;
    }}
    .price.e10 {{
        color: #ff3b30;
    }}
    .updated {{
        font-size: 17px;
        font-weight: 600;
        color: #1c1c1e;
        margin-top: 6px;
    }}
    .footer {{
        font-size: 16px;
        color: #5c5c62;
        font-weight: 600;
        text-align: center;
        margin-top: 6px;
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
