# RiverLens

**自分のハンドを、自分の手元で。オフラインのポーカー振り返りワークスペース。**

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

RiverLens は、Natural8／GGPoker で自分がプレイした完了済みキャッシュゲームのハンド履歴を分析するデスクトップアプリです。React／TypeScript の画面と Rust の解析・統計処理を組み合わせ、SQLite にローカル保存します。

> 公開プレビュー。v0.1.0 はソースコードのみのリリースで、署名済みインストーラーは付属しません。Windows／Intel の実機検証は未完了です。現在の main には外観テーマと追加の README 翻訳が含まれます。README は13言語、アプリの画面は英語・繁体字中国語・簡体字中国語の3言語です。

## 機能

- TXT、ZIP、フォルダーから取り込み。進捗確認、一時停止・再開、重複検出、異常データの隔離。
- 純損益、bb/100、セッション別・ポジション別集計から個別ハンドを確認。
- VPIP、PFR、RFI、3-bet、ブラインド防衛など13項目。分子と機会分母を個別に確認。
- 13 × 13 のスターティングハンド表で件数、net bb、bb/100、アクション頻度を確認。
- アクションを一手ずつ再生し、ストリートを移動。既知のカードを表示。
- メモ、タグ、レビュー済み状態、フィルターを保存。
- ローカル SQLite、CSV／ハンド履歴の書き出し、バックアップと復元。
- Forest、Midnight、Paper（暖白）の外観テーマ。選択を端末に保存。
- 対応するヘッズアップ・単一ポット・既知のホールカード・単一 runout の all-in equity。decision EV や GTO スコアではありません。

## スクリーンショット

Paper テーマ、1920px 幅の全ページキャプチャです。240ハンドの合成データのみを使用し、個人の履歴は含みません。反復サンプルの頻度や利益は現実の成績ではありません。表は観測されたハンドを示し、推奨レンジではありません。

![RiverLens — Paper](docs/screenshots/overview-en.jpg)

![RiverLens — 13 × 13](docs/screenshots/starting-hands-en.jpg)

## はじめに

Node.js 22+、Rust stable、および Tauri のプラットフォーム要件が必要です。macOS は Xcode Command Line Tools、Windows は MSVC C++ Build Tools と WebView2 を使用します。

[Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. PokerCraft から自分の完了済みハンド履歴を TXT／ZIP で書き出します。
2. Data & settings でブランド、Hero 名、履歴本文のタイムゾーンを確認します。
3. Import center で取り込み、集計・ハンド表・リプレイを確認します。
4. メモを保存し、定期的にバックアップします。

## 開発

ターミナル1で Rust core、ターミナル2でブラウザープレビューを起動します。デスクトップ版は Tauri IPC とネイティブファイルダイアログを使用し、プレビューは同じ Rust core に開発用パス入力で接続します。

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

データは Tauri の app-data（app.riverlens.desktop）に保存され、設定画面に実際の場所が表示されます。ブラウザー開発では .local/riverlens.db を使用します。稼働中の SQLite は WAL も扱う内蔵バックアップを利用してください。

## 対象範囲とライセンス

個人のオフライン・セッション後レビュー専用です。ゲームクライアント接続、ライブ HUD、RTA、集団データマイニング、クラウド同期、GTO 最善アクション評価はありません。Natural8／GGPoker とは提携していません。公開されていても、プロジェクト全体の再利用ライセンスは未選定です。RiverLens 自体が MIT／Apache であるとはみなさないでください。第三者コンポーネントは各自のライセンスに従います。

## 資料

[User guide — 繁體中文](docs/user-guide.md) · [Architecture](docs/architecture.md) · [Validation](docs/validation.md) · [Research](docs/research-matrix.md) · [Changelog](CHANGELOG.md) · [Third-party components](THIRD_PARTY.md) · [Releases](https://github.com/G-Loop-co/riverlens/releases)
