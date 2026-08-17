# DC TUNE STUDIO (DCTuneR)

Aplicación avanzada de sintonización para ECU, adaptada y personalizada con interfaz en español bajo la marca **DC TUNE STUDIO**.

Fork de [LibreTune](https://github.com/RallyPat/LibreTune) con soporte multiidioma (Español, English, Português, Magyar).

## Características

* Interfaz y menús en **español** (idioma principal)
* Herramientas de sintonización y visualización en tiempo real
* Soporte para Speeduino, rusEFI, FOME, epicEFI y ECUs compatibles
* AutoTune, datalogging, edición de tablas 2D/3D, plugins WASM, asistente IA

## Requisitos para compilar

- [Rust](https://rustup.rs/) (toolchain estable)
- [Node.js](https://nodejs.org/) 18 o superior + npm
- Dependencias de sistema de [Tauri](https://tauri.app/start/prerequisites/) según tu SO

### Windows
Instala Visual Studio Build Tools (C++) y WebView2 (suele venir con Windows 10/11).

### Linux (Debian/Ubuntu)
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

### macOS
```bash
xcode-select --install
```

## Compilar y ejecutar

```bash
# 1. Ir a la carpeta de la app
cd crates/libretune-app

# 2. Instalar dependencias del frontend
npm install

# 3. Modo desarrollo (hot-reload)
npm run tauri dev

# 4. Generar instaladores (release)
npm run tauri build
```

Los instaladores quedan en:
`crates/libretune-app/src-tauri/target/release/bundle/`

| Plataforma | Archivo típico |
|------------|----------------|
| Windows    | `.msi` / setup NSIS |
| Linux      | `.AppImage` / `.deb` |
| macOS      | `.dmg` / `.app` |

## Cambiar idioma

Ajustes → Language → **Español** (o English / Português / Magyar).  
No hace falta reiniciar.

## Estructura del proyecto

```
DCTuneR_final/
├── crates/
│   ├── libretune-core/     # Núcleo Rust (protocolo ECU, INI, projects…)
│   └── libretune-app/      # App de escritorio Tauri + React
│       ├── src/            # Frontend React + i18n
│       └── src-tauri/      # Backend Tauri
├── docs/                   # Documentación (mdBook)
└── scripts/                # Scripts de build
```

## Licencia

GPL-2.0-only (igual que LibreTune).
