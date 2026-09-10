# RiverLens

**Elleriniz. Verileriniz. Poker oturumlarından sonra inceleme için yerel çalışma alanı.**

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

RiverLens, Natural8 / GGPoker üzerinde oynadığınız tamamlanmış nakit oyun ellerini inceleyen bir masaüstü uygulamasıdır. Arayüz React / TypeScript, ayrıştırma ve istatistik motoru Rust, yerel veri deposu SQLite kullanır.

> Herkese açık önizleme. v0.1.0 yalnızca kaynak kod sürümüdür; imzalı kurulum dosyası içermez. Windows / Intel cihaz doğrulaması tamamlanmamıştır. main, temalar ve README çevirileri ekler. README dosyaları 13 dilde; uygulama arayüzü İngilizce, Geleneksel Çince ve Basitleştirilmiş Çince olmak üzere 3 dildedir.

## Özellikler

- TXT, ZIP veya klasör içe aktarma; ilerleme, duraklatma/devam, yinelenen kayıt tespiti ve hatalı kayıtları ayırma.
- Net sonuçlar, bb/100, oturum ve pozisyon dökümleri üzerinden tek tek ellere erişim.
- VPIP, PFR, RFI, 3-bet, kör bahis savunması ve postflop/showdown sıklıkları dahil 13 istatistik; paylar ve fırsat paydaları ayrı gösterilir.
- 13 × 13 başlangıç eli matrisi: el sayısı, net bb, bb/100 ve eylem sıklığı.
- Eylemleri adım adım oynatma, sokaklar arasında geçiş ve bilinen kartları görüntüleme.
- Notları, etiketleri, inceleme durumunu ve filtreleri kaydetme.
- Yerel SQLite, CSV / el geçmişi dışa aktarma, yedekleme ve geri yükleme.
- Forest, Midnight ve Paper (sıcak beyaz) temaları; seçim yerelde saklanır.
- Desteklenen heads-up, tek pot, bilinen kapalı kartlar ve tek runout durumları için all-in equity. decision EV veya GTO puanı değildir.

## Ekran görüntüleri

Paper temasında, 1920px genişlikte tam sayfa görüntüleridir. Yalnızca 240 sentetik el kullanılır; özel veri yoktur. Tekrarlanan örneklerin sıklıkları ve kazançları gerçek performansı yansıtmaz. Matris gözlemlenen elleri gösterir, önerilen bir range değildir.

![RiverLens — Paper](docs/screenshots/overview-en.jpg)

![RiverLens — 13 × 13](docs/screenshots/starting-hands-en.jpg)

## Başlangıç

Node.js 22+, Rust stable ve Tauri platform araçları gerekir: macOS için Xcode Command Line Tools; Windows için MSVC C++ Build Tools ve WebView2.

[Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. PokerCraft üzerinden kendi tamamlanmış ellerinizi TXT / ZIP olarak dışa aktarın.
2. Data & settings içinde marka, Hero adı ve el geçmişi metnindeki saat dilimini kontrol edin.
3. Import center ile içe aktarın; sonuçları, matrisi ve tekrarları inceleyin.
4. Notları kaydedin ve düzenli yedek alın.

## Geliştirme

Terminal 1’de Rust core, terminal 2’de tarayıcı önizlemesini başlatın. Masaüstü Tauri IPC ve yerel dosya iletişim kutuları kullanır. Önizleme aynı motoru geliştirme amaçlı yol girişiyle kullanır.

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

Veriler Tauri app-data dizinindedir (app.riverlens.desktop); gerçek konum ayarlarda gösterilir. Tarayıcı geliştirmesi .local/riverlens.db kullanır. Etkin SQLite için WAL verilerini de kapsayan yerleşik yedeklemeyi kullanın.

## Kapsam ve lisans

Kişisel, çevrimdışı oturum sonrası inceleme içindir. Oyun istemcisi bağlantısı, canlı HUD, RTA, toplu oyuncu verisi madenciliği, bulut eşitleme veya GTO en iyi eylem puanlaması yoktur. Natural8 / GGPoker ile bağlantılı değildir. Depo herkese açıktır ancak proje genelinde yeniden kullanım lisansı henüz seçilmemiştir. RiverLens için MIT / Apache varsaymayın. Üçüncü taraf bileşenler kendi lisanslarını korur.

## Belgeler

[User guide — 繁體中文](docs/user-guide.md) · [Architecture](docs/architecture.md) · [Validation](docs/validation.md) · [Research](docs/research-matrix.md) · [Changelog](CHANGELOG.md) · [Third-party components](THIRD_PARTY.md) · [Releases](https://github.com/G-Loop-co/riverlens/releases)
