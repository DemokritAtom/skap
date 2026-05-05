# skap

> **Ein Befehl reicht um loszulegen.**

`skap` ist ein schlankes, schnelles CLI-Tool zur Verwaltung von Dev-Projekten unter Linux und macOS – geschrieben in Rust, distribuiert als einzelnes Binary.

```bash
skap new myapp react          # Projekt erstellen
skap start myapp              # Docker starten
skap list                     # Alle Projekte
skap add github myapp         # GitHub Remote anlegen
```

---

## Philosophie

| Prinzip | Bedeutung |
|---|---|
| **Zero friction** | Kein Wizard, keine Pflicht-Fragen beim Erstellen |
| **Sane defaults** | `skap new myapp` → sofort fertig, alles optional |
| **Erweiterbar** | Alles kann später nachträglich per `skap add` hinzugefügt werden |
| **Portabel** | Einzelnes Binary, keine Systemabhängigkeiten |
| **Schnell** | Port-Checks via `TcpListener::bind`, keine externen Tools nötig |

---

## Distribution

| Kanal        | Befehl                                  |
|--------------|-----------------------------------------|
| crates.io    | `cargo install skap`                    |
| npm          | `npm install -g skap`                   |
| Shell        | `curl -fsSL …/install.sh | sh`          |
| GitHub Release | Binary direkt herunterladen           |

**npm Upload:**

Um skap bei npm zu veröffentlichen, benötigst du einen npm-Account und ein Repository mit einer gültigen `package.json` im `npm/`-Verzeichnis. Melde dich mit `npm login` an und führe im `npm/`-Ordner `npm publish --access public` aus. Das Paket erscheint dann auf [npmjs.com](https://www.npmjs.com/).

**Unterstützte Plattformen:**

| Platform      | Target                        |
|--------------|-------------------------------|
| Linux x64    | `x86_64-unknown-linux-gnu`     |
| Linux arm64  | `aarch64-unknown-linux-gnu`    |
| macOS x64    | `x86_64-apple-darwin`          |
| macOS arm64  | `aarch64-apple-darwin`         |

---
## Hilfe & Übersicht

Mit `skap help` oder `skap <Befehl> --help` erhältst du eine vollständige Übersicht aller verfügbaren Befehle, Subcommands und Optionen. Es werden nicht nur die Hauptbefehle angezeigt, sondern auch, welche Argumente und Flags nach jedem Befehl möglich sind. Beispiel:

```bash
skap help
skap add --help
skap new --help
```

So findest du schnell heraus, was du nach jedem Befehl noch angeben kannst (z.B. weitere Subcommands, Flags oder Argumente).

---

## Build from Source

```bash
git clone https://github.com/skap-cli/skap
cd skap
cargo build --release
# Binary liegt in target/release/skap
```

---

## Shell-Integration (`skap open`)

Da ein Kindprozess das `cwd` des Parent-Shells nicht ändern kann, legt man
einmalig eine Funktion in `.bashrc` / `.zshrc` an:

```bash
function skap() {
  if [[ "$1" == "open" ]]; then
    dir=$(command skap open --print-path "$2")
    if [ -n "$dir" ]; then cd "$dir" && command skap open --editor-only "$2"; fi
  else
    command skap "$@"
  fi
}
```

---

## Globale Konfiguration

Alle Konfigurationsdateien liegen unter `~/.config/skap/` und werden beim
ersten Start automatisch angelegt.

| Datei | Inhalt |
|---|---|
| `config.toml` | Editor, Defaults für Git/Docker, Tokens, Base-Port |
| `registry.toml` | Alle registrierten Projekte |
| `ports.toml` | Reservierte Ports je Service |

Konfiguration verwalten:

```bash
skap config set editor vim
skap config set github_token ghp_xxxx
skap config set default_license Apache
skap config set base_port 4000
skap config get editor
skap config list
```

---

## Befehle

### `skap new <name> [template]`

Erstellt ein neues Projekt. Keine interaktiven Fragen – alles läuft mit Defaults durch.

```bash
skap new myapp                        # docker-only Template
skap new myapp react                  # React + Vite
skap new api fastapi --no-docker      # FastAPI ohne Docker
skap new tool rust-cli --no-git --no-docker
skap new shop next --git-remote --private --tag client
skap new service express --port 5000  # Custom Base-Port
```

**Was passiert automatisch:**
- Port-Scan + konfliktfreie Port-Vergabe
- Template-Dateien rendern (Tera Engine)
- `git init` + initialer Commit (konfigurierbar)
- `docker-compose.yml` + `Dockerfile` generieren (konfigurierbar)
- `LICENSE` schreiben (MIT by default)
- Projekt in `registry.toml` registrieren

**Flags:**

| Flag | Bedeutung |
|---|---|
| `--template / -t <name>` | Template explizit wählen |
| `--no-git` | Git überspringen |
| `--no-docker` | Docker überspringen |
| `--git-remote` | Remote auf konfigurierten Provider anlegen |
| `--private` | Private Repo (nur mit `--git-remote`) |
| `--license <name>` | MIT \| Apache \| GPL \| none |
| `--tag <tag>` | Tag vergeben (wiederholbar) |
| `--editor` | Editor nach Erstellung öffnen |
| `--port <n>` | Basis-Port manuell setzen |

**Output:**
```
✓ Projekt "myapp" erstellt  (/home/user/dev/myapp)
✓ Template: react
· Lizenzdatei: MIT
✓ Git initialisiert (initial commit)
✓ Docker Compose generiert (Ports: frontend:3000, backend:8000)
· Registriert in skap registry

→ Starten mit: skap start myapp
```

---

### `skap add <feature> [project]`

Fügt einem bestehenden Projekt nachträglich Features hinzu. Wird kein Projektname angegeben, wird das aktuelle Verzeichnis erkannt.

| Feature | Was passiert |
|---|---|
| `git` | `git init` + `.gitignore` + initialer Commit |
| `docker` | `Dockerfile` + `docker-compose.yml` generieren |
| `github` | GitHub-Remote anlegen (braucht `github_token` in config) |
| `gitlab` | GitLab-Remote anlegen (braucht `gitlab_token` in config) |
| `env` | `.env` + `.env.example` anlegen |
| `lint` | Linter-Config (ESLint / Clippy / Ruff je nach Stack) |
| `ci` | `.github/workflows/ci.yml` generieren |
| `precommit` | `.pre-commit-config.yaml` anlegen |
| `license` | Lizenzdatei hinzufügen (fragt einmalig welche) |
| `readme` | `README.md` generieren |
| `makefile` | `Makefile` mit Standard-Targets |
| `devcontainer` | VS Code Devcontainer Config |
| `db` | Datenbank-Container hinzufügen (fragt einmalig: postgres / mysql / mongo / redis) |
| `ssl` | Self-signed Cert für lokales HTTPS |

```bash
skap add docker myapp
skap add github myapp
skap add db                   # fragt welche DB
skap add ci myapp
```

---

### `skap fix <problem> [project]`

Diagnostiziert und repariert häufige Probleme.

| Problem | Was passiert |
|---|---|
| `ports` | Konflikte erkennen, neue freie Ports vorschlagen, `docker-compose.yml` updaten |
| `env` | Fehlende `.env`-Einträge aus `.env.example` auffüllen |
| `git` | Git-State prüfen |
| `docker` | Compose-Stack neu bauen |
| `permissions` | Dateiberechtigungen normalisieren (world-write entfernen) |
| `deps` | Fehlende Dependencies nachinstallieren (npm / pip / cargo / go) |
| `all` | Alle obigen Checks nacheinander |

```bash
skap fix ports myapp          # fragt einmalig um Bestätigung
skap fix env                  # inferred vom cwd
skap fix all myapp
```

**Port-Fix Ablauf:**
1. Liest `docker-compose.yml` des Projekts
2. Prüft jeden Host-Port (live `TcpListener::bind`)
3. Zeigt Konflikte an + schlägt neue Ports vor
4. Fragt einmalig: `Ports ändern? [Y/n]`
5. Patched `docker-compose.yml` + `ports.toml` + `registry.toml`
6. Startet Container neu falls sie liefen

---

### `skap list`

Alle registrierten Projekte in einer übersichtlichen Tabelle.

```
NAME          TEMPLATE    DOCKER    GIT         PORTS        TAGS
───────────────────────────────────────────────────────────────────
myapp         react       🟢 UP     ✓ remote    3000,8000    client
api-server    fastapi     🔴 ERR    ✓ local     8001
rust-tool     rust-cli    ─          no          ─            intern
old-project   next        ⚪ OFF     ✓ remote    3002
```

Die **GIT**-Spalte zeigt drei klar unterscheidbare Zustände:

| Anzeige (Emoji) | ASCII | Bedeutung |
|---|---|---|
| `✓ remote` | `yes+remote` | Git initialisiert + Remote registriert |
| `✓ local`  | `yes`        | Git initialisiert, kein Remote |
| `no`        | `no`         | Kein Git in diesem Projekt |

```bash
skap list                     # alle aktiven
skap list --tag client        # nach Tag filtern
skap list --running           # nur laufende
skap list --archived          # archivierte einschließen
```

---

### `skap status [project]`

Detailansicht eines einzelnen Projekts.

```
Projekt:    myapp
Pfad:       /home/user/dev/myapp
Template:   react
Erstellt:   2026-04-22

Git:        ✓ sauber  (branch: main, remote: github.com/user/myapp)
Docker:     🟢 läuft
Ports:      3000, 8000
Tags:       client

Disk Usage: 245 MB
```

---

### `skap start / stop / restart [project]`

```bash
skap start myapp              # docker compose up -d
skap stop myapp               # docker compose down
skap restart myapp            # docker compose restart
```

Fehlt Docker im Projekt: Hinweis mit `skap add docker`.

---

### `skap logs [project]`

```bash
skap logs myapp               # letzte 50 Zeilen + follow
skap logs myapp --tail 100
skap logs myapp --service frontend
```

---

### `skap shell [project]`

Öffnet eine Shell in einem laufenden Container. Gibt es mehrere Services, erscheint eine Auswahl.
Läuft das Projekt aktuell **nicht**, fragt skap interaktiv:

```
⚠ Container von 'myapp' laufen aktuell nicht.
? Jetzt starten? [Y/n]
```

Bei `Y` wird `docker compose up -d` ausgeführt und anschließend die Shell geöffnet.

```bash
skap shell myapp
skap shell myapp --service backend
```

---

### `skap run <project> <cmd...>`

Führt einen Befehl im Kontext des Projekts aus – entweder via `docker compose exec` (wenn Container läuft) oder direkt im Projektverzeichnis.
Ist das Projekt als Docker-Projekt registriert, der Container läuft aber nicht, wird der Befehl auf dem Host ausgeführt – mit explizitem Hinweis:

```
⚠ Container läuft nicht – führe Befehl direkt im Projektverzeichnis aus.
```

```bash
skap run myapp npm run build
skap run api python manage.py migrate
skap run tool cargo test
```

---

### `skap rename <old> <new>`

Benennt ein Projekt vollständig um:

- stoppt laufende Container,
- benennt das Verzeichnis um (`mv old new`),
- aktualisiert den Eintrag in `registry.toml`,
- benennt alle Schlüssel `old-*` in `ports.toml` zu `new-*`,
- aktualisiert `.skap.toml` im Projektordner,
- startet die Container danach wieder, falls sie vorher liefen.

```bash
skap rename myapp my-renamed
```

---

### `skap move <project> <new-path>`

Verschiebt ein Projekt an einen anderen Ort im Dateisystem.
Fehlt das Zielverzeichnis, wird der Elternpfad automatisch angelegt.
Laufende Container werden vorher gestoppt und am Zielort neu gestartet.

```bash
skap move api ~/work/backends/api
```

---

### `skap ports list [--used|--free]`

Listet alle in `ports.toml` reservierten Ports und zeigt den **aktuellen** Belegungsstatus auf der Maschine:

```
Service                             Port     Status
────────────────────────────────────────────────────────────
api-api                             3000     🟢 aktiv
web-frontend                        3001     ⚪ frei
```

Mit `--used` werden nur belegte, mit `--free` nur freie Ports angezeigt.
Ist Emoji deaktiviert (`--no-emoji` oder `skap config set emoji false`), werden `USED` / `FREE` ausgegeben.

---

### `skap clone <url> [name]`

Klont ein bestehendes Repo und registriert es bei skap.
Findet sich im Repo eine `.skap.toml`, wird Template- und Port-Information daraus übernommen.
Andernfalls erkennt skap den Stack heuristisch (Next, Vite, FastAPI, Django, Rust, Go, Express, …) und liest – falls vorhanden – Host-Ports aus `docker-compose.yml`.

```bash
skap clone https://github.com/me/cool-app.git
skap clone git@gitlab.com:org/api.git api-prod
```

Ist der abgeleitete Name (Repo-Basename) bereits in der Registry vergeben, **bricht** der Befehl ab und verlangt explizit einen anderen Namen – nichts wird stillschweigend überschrieben:

```
Error: a project named 'cool-app' is already registered –
       pass a different name: `skap clone https://github.com/me/cool-app.git <name>`
```

---

### `skap tag <add|remove|list> ...`

Verwaltet Projekt-Tags nach der Erstellung. Tags werden in `registry.toml` gespeichert und von `skap list --tag <tag>` zur Filterung verwendet.

```bash
skap tag add myapp client            # Tag hinzufügen (idempotent)
skap tag remove myapp client         # Tag entfernen
skap tag list myapp                  # Tags eines Projekts
skap tag list                        # Tags aller Projekte
```

---

### `skap open [project]`

Wechselt ins Projektverzeichnis und öffnet den Editor (siehe Shell-Integration).

```bash
skap open myapp
skap open myapp --editor vim
skap open myapp --no-editor
```

---

### `skap doctor`

Vollständige Systemdiagnose.

```
SYSTEM
  ✓  Docker     29.4.1
  ✓  Git        2.43.0
  ✓  Node       20.11.0
  ✗  Go         not installed
  ✓  Rust       1.95.0

PROJEKTE
  ✓  myapp           alles ok
  ·  rust-tool       stopped
  ✗  old-project     path missing

PORT-KONFLIKTE
  keine

EMPFEHLUNGEN
  → skap archive old-project  (path missing)
```

---

### `skap clean [project]`

```bash
skap clean myapp --images     # Docker Images entfernen
skap clean myapp --volumes    # Docker Volumes entfernen
skap clean myapp --all        # beides
```

---

### `skap archive [project]`

Markiert ein Projekt als archiviert: stoppt laufende Container, blendet es aus `skap list` aus (außer mit `--archived`).

```bash
skap archive old-project
skap list --archived          # zeigt auch archivierte
```

---

### `skap update`

Prüft GitHub Releases auf eine neuere Version und gibt einen Hinweis – das Binary wird **nicht** automatisch ersetzt.

```bash
skap update
# ✓ skap ist aktuell (0.1.0)
```

```bash
skap update
# → Neue Version verfügbar: 0.2.0  (du hast 0.1.0)
#   • cargo install skap
#   • npm i -g skap
#   • curl -fsSL https://skap.dev/install.sh | sh
#
# · Release: https://github.com/skap-cli/skap/releases/tag/v0.2.0
```

---

### `skap config init [--force]`

Legt eine frische `~/.config/skap/config.toml` mit den Default-Werten an.
Existiert die Datei bereits, wird ohne `--force` nachgefragt.
`registry.toml` und `ports.toml` werden **nie** angefasst.

```bash
skap config init
skap config init --force
```

---

## Templates

Alle Templates sind ins Binary kompiliert (kein separater Download nötig).

| Template | Sprache | Services |
|---|---|---|
| `docker-only` | beliebig | app |
| `react` | JavaScript | frontend, backend |
| `next` | TypeScript | app |
| `vue` | JavaScript | frontend |
| `svelte` | JavaScript | frontend |
| `express` | JavaScript | api |
| `fastapi` | Python | api |
| `django` | Python | web |
| `axum` | Rust | api |
| `go-api` | Go | api |
| `rust-cli` | Rust | – (kein Docker) |
| `go-cli` | Go | – (kein Docker) |

**Tera Template-Variablen:**

| Variable | Beispiel |
|---|---|
| `{{ project_name }}` | `my-app` |
| `{{ project_name_pascal }}` | `MyApp` |
| `{{ project_name_snake }}` | `my_app` |
| `{{ frontend_port }}` | `3000` |
| `{{ api_port }}` | `8001` |
| `{{ year }}` | `2026` |
| `{{ author }}` | aus `git config user.name` |
| `{{ license }}` | `MIT` |

---

## Port-Management

Ports werden automatisch zugewiesen – konfliktfrei:

1. Liest `~/.config/skap/ports.toml` (bereits vergebene Ports)
2. Prüft per `TcpListener::bind` ob der Port frei ist (kein `ss` / `netstat` nötig)
3. Startet bei `base_port` (default: 3000) und inkrementiert bis ein freier Port gefunden wird
4. Jeder Service bekommt seinen eigenen Port

```bash
skap config set base_port 4000        # anderen Startport setzen
skap new myapp react --port 5000      # einmalig überschreiben
skap fix ports myapp                  # Konflikte auflösen
```

### Live-Status & Emoji

`skap ports list` zeigt zusätzlich an, ob der Port aktuell auf der Maschine belegt ist (🟢 `aktiv`) oder nicht (⚪ `frei`).
In Terminals ohne UTF-8-Locale, in CI-Umgebungen oder mit explizitem Flag wird automatisch ASCII verwendet:

```bash
skap --no-emoji list           # einmalig ASCII
skap config set emoji false    # dauerhaft ASCII (UP/OFF/ERR, USED/FREE)
```

---

## Konfigurationsdateien

### `~/.config/skap/config.toml`
```toml
[defaults]
editor = "code"
git = true
docker = true
license = "MIT"
git_provider = "github"
github_token = ""
gitlab_token = ""
gitlab_url = "https://gitlab.com"
emoji = true                  # auf false setzen für ASCII-only Output

[ports]
base_port = 3000
```

### `~/.config/skap/registry.toml`
```toml
[projects.myapp]
path = "/home/user/dev/myapp"
template = "react"
created = "2026-04-22T10:00:00Z"
tags = ["client"]
docker = true
git = true
git_remote = "git@github.com:user/myapp.git"
ports = [3000, 8000]
archived = false
```

### `~/.config/skap/ports.toml`
```toml
[ports]
myapp-frontend = 3000
myapp-backend  = 8000
```

### `<projekt>/.skap.toml`

Projektlokales Marker-File, das beim `skap new` und `skap add` automatisch geschrieben wird.
Es gehört **ins Repo** (nicht in `.gitignore`), damit Mitentwickler·innen nach `git clone` direkt `skap doctor` und Co. nutzen können.
`skap clone` liest diese Datei und kennt damit Template + Ports ohne Heuristik.

```toml
[project]
name = "myapp"
template = "react"
created = "2026-04-22T10:00:00Z"

[ports]
frontend = 3000
backend  = 8000
```

---

## Distribution

| Kanal | Befehl |
|---|---|
| crates.io | `cargo install skap` |
| npm | `npm install -g skap` |
| Shell | `curl -fsSL …/install.sh \| sh` |
| GitHub Release | Binary direkt herunterladen |

**Unterstützte Plattformen:**

| Platform | Target |
|---|---|
| Linux x64 | `x86_64-unknown-linux-gnu` |
| Linux arm64 | `aarch64-unknown-linux-gnu` |
| macOS x64 | `x86_64-apple-darwin` |
| macOS arm64 | `aarch64-apple-darwin` |

---

## CI/CD Pipeline

Trigger: `git tag v*`

1. Cross-compile für alle 4 Targets via `cross`
2. Binaries als GitHub Release Assets hochladen
3. `cargo publish` → crates.io
4. `npm publish` → npm-Registry

---

## Roadmap

Noch **nicht** implementiert (geplant):

- TUI Dashboard (`ratatui`)
- Secrets-Management / verschlüsselte `.env`
- Community Template Registry
- Tmux / Zellij Integration
- Automatisches Background-Fetching
- PR/MR Status via API

Ausdrücklich **nicht** geplant:

- **Windows Support** – skap geht stark auf Unix-Tooling (POSIX-Shells, `docker compose`, `~/.config`, Symlinks, Signal-Handling) zurück. Eine native Windows-Portierung ist aktuell nicht vorgesehen. WSL2 funktioniert wie ein normales Linux.

