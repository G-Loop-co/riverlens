<div align="center">

# RiverLens

**Tus manos. Tus datos. Un espacio local para revisar tu póker después de cada sesión.**

Importa tus propios historiales desde más de 20 salas, estudia tendencias de la población, encuentra fugas con evidencia real y fíjalas con práctica — todo en local, en tu máquina.

[![Release](https://img.shields.io/github/v/release/G-Loop-co/riverlens)](https://github.com/G-Loop-co/riverlens/releases/latest)
[![Desktop CI](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml/badge.svg)](https://github.com/G-Loop-co/riverlens/actions/workflows/desktop.yml)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%28Apple%20Silicon%20%26%20Intel%29-blue)
![Runtime](https://img.shields.io/badge/runtime-offline%20first-2f6f4f)

**[⬇️ Descargar RiverLens v0.6.1](https://github.com/G-Loop-co/riverlens/releases/latest)**

<sub>Las builds de macOS están firmadas ad-hoc, sin notarización.</sub>

![RiverLens overview — synthetic data](docs/screenshots/overview-en.jpg)

</div>


[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

---

## Funciones

- Importa TXT, ZIP o carpetas; revisa el progreso, pausa/reanuda, detecta duplicados y aísla registros anómalos.
- Resultados netos, bb/100 y desgloses por sesión o posición con acceso a cada mano.
- 13 estadísticas: VPIP, PFR, RFI, 3-bet, defensa de ciegas y frecuencias postflop/showdown con numeradores y denominadores de oportunidades explícitos.
- Matriz inicial de 13 × 13 con número de manos, net bb, bb/100 y frecuencia de acción.
- Reproduce acciones paso a paso, cambia de calle y consulta las cartas conocidas.
- Guarda notas, etiquetas, estado de revisión y filtros.
- SQLite local, exportación CSV / historiales, copias de seguridad y restauración.
- Temas Forest, Midnight y Paper (blanco cálido), con selección guardada localmente.
- Equity all-in para casos compatibles: heads-up, un solo bote, cartas privadas conocidas y un solo runout. No es decision EV ni una puntuación GTO.

## Capturas de pantalla

Capturas de página completa con tema Paper y anchura de 1920px. Solo contienen 240 manos sintéticas, sin datos privados. Las frecuencias y ganancias de la muestra repetitiva no son resultados reales. La matriz muestra manos observadas, no un rango recomendado.

![RiverLens — Paper](docs/screenshots/overview-en.jpg)

![RiverLens — 13 × 13](docs/screenshots/starting-hands-en.jpg)

## Primeros pasos

Requiere Node.js 22+, Rust stable y las herramientas de plataforma de Tauri: Xcode Command Line Tools en macOS; MSVC C++ Build Tools y WebView2 en Windows.

[Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. Exporta tus manos terminadas de PokerCraft como TXT / ZIP.
2. En Data & settings, confirma la marca, el nombre Hero y la zona horaria del texto del historial.
3. Importa en Import center y revisa resultados, matriz y reproducciones.
4. Guarda notas y realiza copias de seguridad periódicas.

## Desarrollo

Ejecuta Rust core en el terminal 1 y la vista previa del navegador en el terminal 2. La aplicación usa Tauri IPC y diálogos nativos; la vista previa emplea el mismo motor con entrada de rutas para desarrollo.

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

Los datos se guardan en app-data de Tauri (app.riverlens.desktop); los ajustes muestran la ruta real. El navegador de desarrollo usa .local/riverlens.db. Para SQLite activo, utiliza la copia de seguridad integrada que incluye los datos WAL.

## Alcance y licencia

Para revisión personal sin conexión después de la sesión. No incluye conexión al cliente, HUD en vivo, RTA, sincronización en la nube ni evaluación GTO de la mejor acción. Sin afiliación a Natural8 / GGPoker. El repositorio es público, pero aún no se ha elegido una licencia general de reutilización. No supongas que RiverLens tiene licencia MIT / Apache. Los componentes de terceros conservan sus licencias.

## Documentación

[User guide — 繁體中文](docs/user-guide.md) · [Architecture](docs/architecture.md) · [Validation](docs/validation.md) · [Research](docs/research-matrix.md) · [Changelog](CHANGELOG.md) · [Third-party components](THIRD_PARTY.md) · [Releases](https://github.com/G-Loop-co/riverlens/releases)
