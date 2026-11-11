<div align="center">

# RiverLens

**Le tue mani. I tuoi dati. Uno spazio locale per il review post-sessione.**

Importa i tuoi storici da oltre 20 sale, studia le tendenze della popolazione, trova i leak con prove reali e fissali con l'allenamento — tutto in locale, sul tuo computer.

[![Release](https://img.shields.io/github/v/release/G-Loop-co/riverlens)](https://github.com/G-Loop-co/riverlens/releases/latest)
[![Desktop CI](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml/badge.svg)](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%28Apple%20Silicon%20%26%20Intel%29-blue)
![Runtime](https://img.shields.io/badge/runtime-offline%20first-2f6f4f)

**[⬇️ Scarica RiverLens v0.6.1](https://github.com/G-Loop-co/riverlens/releases/latest)**

<sub>Le build macOS sono firmate ad-hoc, non notarizzate.</sub>

![RiverLens overview — synthetic data](docs/screenshots/overview-en.jpg)

</div>


[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

---

## Funzioni

- Importa TXT, ZIP o cartelle; controlla avanzamento, pausa/ripresa, duplicati e quarantena dei record anomali.
- Risultati netti, bb/100 e riepiloghi per sessione o posizione, con accesso alle singole mani.
- 13 statistiche: VPIP, PFR, RFI, 3-bet, difesa dei bui e frequenze postflop/showdown con numeratori e denominatori delle opportunità espliciti.
- Matrice iniziale 13 × 13: numero di mani, net bb, bb/100 e frequenza delle azioni.
- Replay azione per azione, cambio street e carte note.
- Salva note, tag, stato di revisione e filtri.
- SQLite locale, esportazione CSV / mani, backup e ripristino.
- Temi Forest, Midnight e Paper (bianco caldo), con scelta salvata localmente.
- Equity all-in nei casi supportati: heads-up, piatto singolo, carte private note e runout singolo. Non è decision EV né un punteggio GTO.

## Schermate

Acquisizioni dell’intera pagina nel tema Paper, larghe 1920px. Usano solo 240 mani sintetiche, senza dati privati. Frequenze e guadagni del campione ripetitivo non rappresentano risultati reali. La matrice mostra mani osservate, non un range consigliato.

![RiverLens — Paper](docs/screenshots/overview-en.jpg)

![RiverLens — 13 × 13](docs/screenshots/starting-hands-en.jpg)

## Per iniziare

Servono Node.js 22+, Rust stable e gli strumenti di piattaforma Tauri: Xcode Command Line Tools su macOS; MSVC C++ Build Tools e WebView2 su Windows.

[Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. Esporta da PokerCraft le tue mani concluse in TXT / ZIP.
2. In Data & settings verifica marchio, nome Hero e fuso orario nel testo delle mani.
3. Importa in Import center e consulta risultati, matrice e replay.
4. Salva le note ed esegui backup regolari.

## Sviluppo

Avvia Rust core nel terminale 1 e l’anteprima browser nel terminale 2. Il desktop usa Tauri IPC e finestre native; l’anteprima impiega lo stesso motore con inserimento dei percorsi per lo sviluppo.

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

I dati sono nella directory app-data Tauri (app.riverlens.desktop); le impostazioni mostrano il percorso effettivo. Lo sviluppo browser usa .local/riverlens.db. Per SQLite attivo usa il backup integrato, che gestisce anche il WAL.

## Ambito e licenza

Per revisione personale offline dopo la sessione. Nessun collegamento al client, HUD live, RTA, sincronizzazione cloud o valutazione GTO della migliore azione. Nessuna affiliazione a Natural8 / GGPoker. Il repository è pubblico, ma non è stata scelta una licenza generale di riutilizzo. Non presumere MIT / Apache per RiverLens stesso. I componenti di terze parti mantengono le proprie licenze.

## Documentazione

[User guide — 繁體中文](docs/user-guide.md) · [Architecture](docs/architecture.md) · [Validation](docs/validation.md) · [Research](docs/research-matrix.md) · [Changelog](CHANGELOG.md) · [Third-party components](THIRD_PARTY.md) · [Releases](https://github.com/G-Loop-co/riverlens/releases)
