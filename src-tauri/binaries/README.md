# Dossier des Binaires Sidecar FFmpeg

Ce dossier accueille les exécutables autonomes de FFmpeg requis pour le transcodage vidéo sur chaque système d'exploitation cible :

- **Windows x86_64 :** `ffmpeg-x86_64-pc-windows-msvc.exe`
- **Linux x86_64 :** `ffmpeg-x86_64-unknown-linux-gnu`
- **macOS Intel :** `ffmpeg-x86_64-apple-darwin`
- **macOS Apple Silicon :** `ffmpeg-aarch64-apple-darwin`

Lors de la compilation avec Tauri v2 (`npm run tauri build`), le bundler sélectionne automatiquement le binaire correspondant au triplet de compilation cible.
