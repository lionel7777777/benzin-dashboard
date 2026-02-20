use axum::{routing::get, Router};

async fn dashboard() -> &'static str {
    r#"
    <html>
    <head>
    <meta http-equiv="refresh" content="60">
    </head>
    <body style="background:black;color:white;text-align:center">
        <h1>Tankstelle</h1>
        <h2 style="font-size:80px">Preis lädt...</h2>
    </body>
    </html>
    "#
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(dashboard));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}