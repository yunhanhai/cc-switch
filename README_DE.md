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

Die `3.20.1` in den Quelldateien bezeichnet den übernommenen Versionsstand. Sie ist keine eigenständige veröffentlichte Versionsnummer dieses Projekts.
Derzeit stehen auf GitHub Releases keine Pakete dieses Projekts zur Verfügung. Für die Entwicklung wird die Anwendung aus dem Quellcode gestartet.
Installationspakete und Paketmanager-Befehle des Ursprungsprojekts liefern nicht die Erweiterungen dieses Repositorys.

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
