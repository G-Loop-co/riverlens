<div align="center">

# RiverLens

**Vos mains. Vos données. Un espace local pour l'analyse post-session.**

Importez vos propres historiques depuis plus de 20 salles, étudiez les tendances de la population, trouvez vos fuites avec de vraies preuves et ancrez-les par le drill — tout en local, sur votre machine.

[![Release](https://img.shields.io/github/v/release/G-Loop-co/riverlens)](https://github.com/G-Loop-co/riverlens/releases/latest)
[![Desktop CI](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml/badge.svg)](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%28Apple%20Silicon%20%26%20Intel%29-blue)
![Runtime](https://img.shields.io/badge/runtime-offline%20first-2f6f4f)

**[⬇️ Télécharger RiverLens v0.6.0](https://github.com/G-Loop-co/riverlens/releases/latest)**

<sub>Les builds macOS sont signés ad-hoc, non notarized.</sub>

![RiverLens overview — synthetic data](docs/screenshots/overview-en.jpg)

</div>


[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

---

## Fonctionnalités

- Import TXT, ZIP ou dossiers : progression, pause/reprise, doublons et mise en quarantaine des anomalies.
- Résultats nets, bb/100 et ventilations par session ou position, avec accès aux mains correspondantes.
- 13 statistiques : VPIP, PFR, RFI, 3-bet, défense des blindes et fréquences postflop/abattage, avec numérateurs et dénominateurs d’opportunités explicites.
- Matrice de départ 13 × 13 : nombre de mains, net bb, bb/100 et fréquence d’action.
- Relecture action par action, navigation entre les streets et cartes connues.
- Notes, tags, état de révision et filtres enregistrés.
- SQLite local, exports CSV / historiques, sauvegarde et restauration.
- Thèmes Forest, Midnight et Paper (blanc chaud), avec mémorisation locale.
- Équité all-in pour les cas compatibles : heads-up, pot unique, cartes privatives connues et runout unique. Ce n’est ni la decision EV ni un score GTO.

## Captures d’écran

Captures de page entière en thème Paper, sur une largeur de 1920px. Elles utilisent uniquement 240 mains synthétiques, sans données privées. Les fréquences et gains de cet échantillon répétitif ne représentent pas des performances réelles. La matrice décrit les mains observées, pas une range recommandée.

![RiverLens — Paper](docs/screenshots/overview-en.jpg)

![RiverLens — 13 × 13](docs/screenshots/starting-hands-en.jpg)

## Démarrage

Prérequis : Node.js 22+, Rust stable et outils de plateforme Tauri. Sur macOS : Xcode Command Line Tools ; sur Windows : MSVC C++ Build Tools et WebView2.

[Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. Exportez vos propres mains terminées depuis PokerCraft au format TXT / ZIP.
2. Dans Data & settings, vérifiez la marque, le nom Hero et le fuseau horaire du texte des historiques.
3. Importez dans Import center, puis explorez les résultats, la matrice et les relectures.
4. Enregistrez vos notes et sauvegardez régulièrement.

## Développement

Lancez le moteur Rust dans le terminal 1 et l’aperçu navigateur dans le terminal 2. L’application de bureau utilise Tauri IPC et les dialogues natifs ; l’aperçu emploie le même moteur et une saisie de chemins réservée au développement.

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

Les données résident dans le dossier app-data Tauri (app.riverlens.desktop), dont le chemin apparaît dans les paramètres. Le développement navigateur utilise .local/riverlens.db. Pour une base SQLite active, utilisez la sauvegarde intégrée afin de prendre en compte le WAL.

## Périmètre et licence

Usage personnel, hors ligne, après la session. Pas de connexion au client de jeu, HUD en direct, RTA, synchronisation cloud ou évaluation GTO de la meilleure action. Aucune affiliation à Natural8 / GGPoker. Le dépôt est public, mais aucune licence globale de réutilisation n’a été choisie : ne présumez pas que RiverLens est sous MIT / Apache. Les composants tiers conservent leurs licences.

## Documentation

[User guide — 繁體中文](docs/user-guide.md) · [Architecture](docs/architecture.md) · [Validation](docs/validation.md) · [Research](docs/research-matrix.md) · [Changelog](CHANGELOG.md) · [Third-party components](THIRD_PARTY.md) · [Releases](https://github.com/G-Loop-co/riverlens/releases)
