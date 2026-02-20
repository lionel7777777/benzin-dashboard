# Benzin-Dashboard

Kleine Web-App, die die **aktuellen Tankpreise deiner Tankstelle** per [Tankerkönig-API](https://creativecommons.tankerkoenig.de/) anzeigt und sich für das Hosting auf **Leapcell** eignet (von überall erreichbar).

## Lokal testen

```bash
# Ohne API: Platzhalter „Meine Tankstelle“ und „– €“
cargo run --release

# Mit Tankerkönig (optional):
export TANKERKOENIG_API_KEY="dein-api-key"
export TANKERKOENIG_STATION_ID="uuid-deiner-tankstelle"
cargo run --release
```

Dann im Browser: [http://127.0.0.1:8080](http://127.0.0.1:8080)

## Tankerkönig einrichten

1. **API-Key:** Kostenlos registrieren unter [creativecommons.tankerkoenig.de](https://creativecommons.tankerkoenig.de/) und API-Key holen.
2. **Tankstellen-ID (UUID):** Auf [TankstellenFinder](https://creativecommons.tankerkoenig.de/TankstellenFinder/index.html) deine Tankstelle suchen, anklicken – die ID steht in der URL oder in den Stationsdaten.

## Auf Leapcell hosten (von überall aufrufbar)

1. **Repository auf GitHub** pushen (z. B. dieses Projekt).
2. Auf [Leapcell](https://leapcell.io/) gehen, mit GitHub anmelden und **New Service** anlegen.
3. **Repository** auswählen und folgende Einstellungen setzen:

   | Feld            | Wert                              |
   |-----------------|------------------------------------|
   | Runtime         | Rust (Any version)                |
   | Build Command   | `cargo build --release`           |
   | Start Command   | `./target/release/benzin-dashboard` |
   | Port            | `8080`                            |

4. **Umgebungsvariablen** im Leapcell-Dashboard setzen (unter dem Service → Environment):
   - `TANKERKOENIG_API_KEY` = dein Tankerkönig-API-Key
   - `TANKERKOENIG_STATION_ID` = UUID deiner Tankstelle

5. **Deploy** starten. Danach erreichst du die Seite unter einer URL wie `dein-service.leapcell.dev`.

Die Seite aktualisiert sich automatisch alle 60 Sekunden (Meta-Refresh).
