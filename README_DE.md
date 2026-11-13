# YunSwitch

Konfigurationsverwaltung und lokales Routing für Claude Code, Codex, Gemini CLI und weitere KI-Werkzeuge, mit laufenden Verbesserungen für mehrere Anbieter.

**Maintainer: yunhanhai** · **Lizenz: [MIT](LICENSE)**

[English](README.md) | [中文](README_ZH.md) | [日本語](README_JA.md) | Deutsch

## Über dieses Projekt

Dieses Repository ist eine eigenständig gepflegte Weiterentwicklung von [farion1231/cc-switch](https://github.com/farion1231/cc-switch), ausgehend von **v3.20.1**.
yunhanhai übernimmt die Pflege und erweitert die vorhandene Konfigurationsverwaltung und Protokollumsetzung insbesondere um Routing und Fehlerbehandlung für mehrere Anbieter.

Künftig sollen auch Ansätze aus weiteren Open-Source-Projekten in die Entwicklung einfließen.
Herkunft und Lizenz übernommener Codebestandteile werden dokumentiert; Änderungen und Veröffentlichungen dieses Projekts werden eigenständig verwaltet.

- [Repository auf Gitee](https://gitee.com/yunhanhai/yunswitch)
- [Repository auf GitHub](https://github.com/yunhanhai/yunswitch)
- [Herkunft, Versionierung und Pflegeplan](docs/PROJECT.md)

## Erweiterungen in diesem Repository

| Bereich          | Erweiterung                                                                                                      |
| ---------------- | ---------------------------------------------------------------------------------------------------------------- |
| Routinggruppen   | Anbieter je Anwendung gruppieren und sortieren; Vorschau der wirksamen Route mit Statusanzeige                   |
| Modell-Aliase    | Einen logischen Modellnamen je Anbieter auf dessen tatsächlichen Modellnamen abbilden, auch beim Anbieterwechsel |
| Gemini-Routing   | Modellnamen im URL-Pfad passend zum gewählten Anbieter umschreiben                                               |
| Modellkatalog    | Modelllisten der Anbieter, ergänzende Daten und einen Cache zur Konfiguration und Zuordnung nutzen               |
| Claude → Codex   | API-Einstellungen und Modellnamen übertragen, doppelte Einträge vermeiden und Betriebsbedingungen anzeigen       |
| Fehlerbehandlung | Wiederholungen beim gleichen Anbieter, Wartezeiten bei HTTP 429, Abkühlzeiten und Failover-Anzeigen erweitern    |

Die Konvertierung von Einstellungen nutzt die aus dem Ursprungsprojekt übernommene Protokollumsetzung.
Im Bridge-Modus muss das lokale Routing vor der Nutzung gestartet werden.
Die Auswahl einer direkten Verbindung beruht auf der erreichbaren Modellliste und dem Vorhandensein des ursprünglichen Modells; sie prüft die Funktion von `/responses` noch nicht.

Wiederholungen beim gleichen Anbieter sind standardmäßig ausgeschaltet. Sie gelten für Verbindungsfehler und bestimmte vorübergehende HTTP-Fehler; ein bereits an den Client übertragener Stream wird nicht erneut gesendet.

## Versionsstand und Bezug

Die Windows-x64-Vorabversion `3.20.1-yunswitch.6` ist als Installer und portable ZIP verfügbar: [GitHub](https://github.com/yunhanhai/yunswitch/releases/tag/v3.20.1-yunswitch.6) / [Gitee](https://gitee.com/yunhanhai/yunswitch/releases/tag/v3.20.1-yunswitch.6). Installation, Start, simulierte Proxy-Anfragen, Migration synthetischer Daten, Upgrade von `.5` und Deinstallation wurden geprüft. Die Pakete sind nicht signiert; automatische Updates sind deaktiviert. Die Upstream-Basis bleibt v3.20.1.
Siehe [Veröffentlichungsnachweis](docs/releases/3.20.1-yunswitch.6.md) und [Migrationsanleitung](docs/MIGRATION-YUNSWITCH.md). Die Entwicklungsversion verwendet `~/.yunswitch-dev`, die Veröffentlichungsversion `~/.yunswitch`.
Installationspakete und Paketmanager-Befehle des Ursprungsprojekts liefern nicht die Erweiterungen dieses Repositorys.

Version `.6` decodiert komprimierte Codex-Responses schrittweise und gibt Wartezustände und Circuit-Probes bei einem Abbruch frei. Erfolgreiche parallele Anfragen löschen keine fremden Warteanzeigen. Nach Ausgabe oder Tool-Aktivität wird nicht erneut gesendet. Portable Version und MSI-Upgrade wurden geprüft. Siehe [Wiederherstellungsgrenzen](docs/CODEX-STREAM-RECOVERY.md) und [Veröffentlichungsnachweis `.6`](docs/releases/3.20.1-yunswitch.6.md).

Die vorherige [Version `.2`](docs/releases/3.20.1-yunswitch.2.md) entfernte übernommene Empfehlungslinks und kommerzielle Partnerkennzeichnungen aus eingebauten Vorlagen. Die Standardsortierung zeigt offizielle Vorlagen zuerst und ordnet die übrigen nach Anzeigename; erforderliche Kennungen und vorhandene Benutzerkonfigurationen bleiben erhalten. Die erste Vorabversion und ihre Prüfungshistorie stehen im [`.1`-Veröffentlichungsnachweis](docs/releases/3.20.1-yunswitch.1.md).

| Plattform     | Prüfung                                                                                                                                                                                                                                                  | Veröffentlichungsstand                                     |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Windows x64   | [Installationsprüfung auf einem frischen Runner](https://github.com/yunhanhai/yunswitch/actions/runs/43445087046): portable ZIP und MSI, einschließlich Oberflächenstart, simulierter Proxy-Anfragen, Migration synthetischer Daten und Upgrade von `.5` | Eigenständige Vorabversion verfügbar                       |
| macOS / Linux | [Quellcode-CI](https://github.com/yunhanhai/yunswitch/actions/runs/43445225978): Frontend-Prüfungen, plattformspezifische Backend-Tests und Clippy                                                                                                       | Noch keine eigenständigen Pakete oder Installationsabnahme |
| ARM64         | Noch keine eigenständige Installationsabnahme                                                                                                                                                                                                            | Keine eigenständigen Pakete veröffentlicht                 |

Diese Nachweise betreffen Quellcode und Pakete der Vorabversion `.6`. Vorlagen dienen als Ausgangspunkt für die Konfiguration. Echte Verbindungen zu allen kostenpflichtigen Anbietern und Migrationen privater Daten wurden nicht umfassend geprüft. Siehe [Prüfumfang](docs/PROJECT.md#供应商目录维护与验证范围).

## Entwicklung aus dem Quellcode

Benötigt werden Node.js, pnpm, Rust und die Tauri-Entwicklungsumgebung für das jeweilige Betriebssystem.
Die Einzelheiten stehen im [Entwicklungs- und Beitragsleitfaden](CONTRIBUTING.md).

```bash
git clone https://gitee.com/yunhanhai/yunswitch.git
cd yunswitch
pnpm install --frozen-lockfile
pnpm dev
```

## Dokumentation

- [Chinesisches README](README_ZH.md): ausführliche Funktionen und Entwicklungshinweise
- [Routinggruppen auf Englisch](docs/routing-groups.en.md): Bedienung und wirksame Route
- [Entwurf der Modell-Aliase](docs/model-alias-routing-design.md): technische Dokumentation auf Chinesisch
- [Bisherige Entwicklungsnotizen](docs/RELEASE-NOTES-P1-P6.md): historischer Release-Entwurf aus der früheren Entwicklungsumgebung; Testangaben sind historische Ergebnisse
- [Beiträge](CONTRIBUTING.md): Fehlerberichte, Vorschläge und Entwicklungsablauf

## Herkunft und Lizenz

Vielen Dank an Jason Young und die Mitwirkenden des ursprünglichen CC Switch.
Der ursprüngliche Copyright-Vermerk und die MIT-Lizenz bleiben in [LICENSE](LICENSE) erhalten.
yunhanhai verantwortet die Erweiterungen und Pflege dieses Repositorys. Die übernommenen Grundlagen und die eigenen Änderungen sind in der Git-Historie und der Projektdokumentation nachvollziehbar.
