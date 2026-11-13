<div align="center">

# RiverLens

**Deine Hände. Deine Daten. Ein lokaler Workspace für Poker-Review nach der Session.**

Importiere eigene Hand-Histories aus 20+ Räumen, studiere Populationstendenzen, finde Leaks mit echten Belegen und verankere sie im Training — komplett offline, auf deinem Rechner.

[![Release](https://img.shields.io/github/v/release/G-Loop-co/riverlens)](https://github.com/G-Loop-co/riverlens/releases/latest)
[![Desktop CI](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml/badge.svg)](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%28Apple%20Silicon%20%26%20Intel%29-blue)
![Runtime](https://img.shields.io/badge/runtime-offline%20first-2f6f4f)

**[⬇️ RiverLens v0.6.2 herunterladen](https://github.com/G-Loop-co/riverlens/releases/latest)**

<sub>macOS-Builds sind ad-hoc signiert, nicht notarized.</sub>

![RiverLens overview — synthetic data](docs/screenshots/overview-en.jpg)

</div>


[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

---

## Funktionen

- TXT, ZIP oder Ordner importieren: Fortschritt, Pause/Fortsetzen, Duplikate und Quarantäne fehlerhafter Datensätze.
- Nettoergebnis, bb/100 und Aufschlüsselungen nach Session oder Position mit Zugriff auf einzelne Hände.
- 13 Kernstatistiken: VPIP, PFR, RFI, 3-bet, Blind Defense sowie Postflop-/Showdown-Häufigkeiten mit getrennten Zählern und Gelegenheitsnennern.
- 13 × 13 Starthandmatrix für Handanzahl, net bb, bb/100 und Aktionshäufigkeit.
- Aktionen schrittweise wiedergeben, Streets wechseln und bekannte Karten anzeigen.
- Notizen, Tags, Prüfstatus und Filter speichern.
- Lokales SQLite, CSV-/Handexport, Sicherung und Wiederherstellung.
- Designs Forest, Midnight und Paper (Warmweiß), lokal gespeicherte Auswahl.
- All-in-Equity für unterstützte Heads-up-Situationen mit einem Pot, bekannten Hole Cards und einem Runout. Keine decision EV oder GTO-Bewertung.

## Screenshots

Vollständige Seitenaufnahmen im Paper-Design bei 1920px Breite. Nur 240 synthetische Hände, keine privaten Daten. Häufigkeiten und Gewinne dieser repetitiven Stichprobe sind keine realen Spielergebnisse. Die Matrix zeigt beobachtete Hände, keine empfohlene Range.

![RiverLens — Paper](docs/screenshots/overview-en.jpg)

![RiverLens — 13 × 13](docs/screenshots/starting-hands-en.jpg)

## Erste Schritte

Benötigt werden Node.js 22+, Rust stable und die Tauri-Plattformwerkzeuge: Xcode Command Line Tools auf macOS; MSVC C++ Build Tools und WebView2 auf Windows.

[Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. Eigene abgeschlossene Hände aus PokerCraft als TXT / ZIP exportieren.
2. Unter Data & settings Marke, Hero-Name und Zeitzone im Handverlauf prüfen.
3. Im Import center importieren und Ergebnisse, Matrix und Replays ansehen.
4. Notizen speichern und regelmäßig sichern.

## Entwicklung

Rust core in Terminal 1 und die Browservorschau in Terminal 2 starten. Die Desktop-App nutzt Tauri IPC und native Dateidialoge. Die Vorschau verwendet denselben Kern mit einer Pfadeingabe für die Entwicklung.

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

Daten liegen im Tauri-app-data-Verzeichnis (app.riverlens.desktop); die Einstellungen zeigen den tatsächlichen Pfad. Die Browservorschau nutzt .local/riverlens.db. Bei einer aktiven SQLite-Datenbank die integrierte Sicherung verwenden, damit WAL-Daten berücksichtigt werden.

## Umfang und Lizenz

Für persönliche Offline-Nachanalyse. Keine Spielclient-Verbindung, Live-HUD, RTA, Cloud-Synchronisierung oder GTO-Bewertung der besten Aktion. Keine Verbindung zu Natural8 / GGPoker. Das Repository ist öffentlich, aber eine projektweite Wiederverwendungslizenz wurde noch nicht gewählt. MIT / Apache gilt nicht automatisch für RiverLens selbst. Drittanbieter behalten ihre jeweiligen Lizenzen.

## Dokumentation

[User guide — 繁體中文](docs/user-guide.md) · [Architecture](docs/architecture.md) · [Validation](docs/validation.md) · [Research](docs/research-matrix.md) · [Changelog](CHANGELOG.md) · [Third-party components](THIRD_PARTY.md) · [Releases](https://github.com/G-Loop-co/riverlens/releases)
