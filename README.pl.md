# RiverLens

**Twoje rozdania. Twoje dane. Lokalna przestrzeń do analizy pokera po sesji.**

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

RiverLens to aplikacja desktopowa do analizy własnych zakończonych rozdań cash game z Natural8 / GGPoker. Interfejs korzysta z React / TypeScript, silnik parsowania i statystyk z Rust, a dane są przechowywane lokalnie w SQLite.

> Publiczna wersja podglądowa. v0.1.0 udostępnia wyłącznie kod źródłowy, bez podpisanych instalatorów. Weryfikacja na urządzeniach Windows / Intel pozostaje niepełna. main dodaje motywy i tłumaczenia README. README są w 13 językach; interfejs aplikacji nadal obsługuje angielski oraz chiński tradycyjny i uproszczony.

## Funkcje

- Import TXT, ZIP lub folderów; postęp, pauza/wznowienie, wykrywanie duplikatów i izolowanie błędnych rekordów.
- Wynik netto, bb/100 oraz zestawienia według sesji i pozycji z dostępem do rozdań.
- 13 statystyk: VPIP, PFR, RFI, 3-bet, obrona blindów i częstotliwości postflop/showdown, z osobnymi licznikami i mianownikami okazji.
- Macierz rąk startowych 13 × 13: liczba rozdań, net bb, bb/100 i częstotliwość akcji.
- Odtwarzanie akcji krok po kroku, przechodzenie między ulicami i znane karty.
- Zapisywanie notatek, tagów, statusu przeglądu i filtrów.
- Lokalny SQLite, eksport CSV / historii, kopie zapasowe i przywracanie.
- Motywy Forest, Midnight i Paper (ciepła biel), z lokalnym zapisem wyboru.
- Equity all-in dla obsługiwanych sytuacji heads-up: jedna pula, znane karty własne i pojedynczy runout. To nie decision EV ani ocena GTO.

## Zrzuty ekranu

Pełne strony w motywie Paper, przy szerokości 1920px. Wyłącznie 240 syntetycznych rozdań, bez prywatnych danych. Częstotliwości i zyski powtarzalnej próbki nie przedstawiają rzeczywistych wyników. Macierz pokazuje zaobserwowane ręce, nie zalecany zakres.

![RiverLens — Paper](docs/screenshots/overview-en.jpg)

![RiverLens — 13 × 13](docs/screenshots/starting-hands-en.jpg)

## Pierwsze kroki

Wymagania: Node.js 22+, Rust stable i narzędzia platformy Tauri: Xcode Command Line Tools na macOS; MSVC C++ Build Tools oraz WebView2 na Windows.

[Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. Wyeksportuj własne zakończone rozdania z PokerCraft jako TXT / ZIP.
2. W Data & settings sprawdź markę, nazwę Hero i strefę czasową w treści historii.
3. Zaimportuj przez Import center, a następnie przeglądaj wyniki, macierz i powtórki.
4. Zapisuj notatki i regularnie twórz kopie zapasowe.

## Rozwój

Uruchom Rust core w terminalu 1, a podgląd przeglądarkowy w terminalu 2. Desktop korzysta z Tauri IPC i natywnych okien plików; podgląd używa tego samego silnika z wprowadzaniem ścieżek na potrzeby rozwoju.

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

Dane są w katalogu app-data Tauri (app.riverlens.desktop); ustawienia pokazują rzeczywistą ścieżkę. Podgląd używa .local/riverlens.db. Dla aktywnego SQLite używaj wbudowanej kopii zapasowej, która uwzględnia WAL.

## Zakres i licencja

Do osobistej analizy offline po sesji. Bez połączenia z klientem gry, HUD na żywo, RTA, eksploracji danych populacji, synchronizacji chmurowej i oceny najlepszej akcji GTO. Brak powiązania z Natural8 / GGPoker. Repozytorium jest publiczne, lecz nie wybrano ogólnej licencji ponownego wykorzystania. Nie zakładaj MIT / Apache dla samego RiverLens. Komponenty zewnętrzne zachowują własne licencje.

## Dokumentacja

[User guide — 繁體中文](docs/user-guide.md) · [Architecture](docs/architecture.md) · [Validation](docs/validation.md) · [Research](docs/research-matrix.md) · [Changelog](CHANGELOG.md) · [Third-party components](THIRD_PARTY.md) · [Releases](https://github.com/G-Loop-co/riverlens/releases)
