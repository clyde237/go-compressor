# GO COMPRESSOR (OptiMedia Desktop)

[![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue?logo=tauri)](https://tauri.app/)
[![React 19](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-blue?logo=typescript)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-DEA584?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)]()
[![License](https://img.shields.io/badge/License-Proprietary-red)]()

**GO COMPRESSOR** est une application de bureau multiplateforme haute performance dédiée à l'optimisation et à la compression par lots d'images et de vidéos.

Conçue sur une architecture moderne couplant **Tauri v2**, **React.js 19 (TypeScript)** et un moteur backend natif en **Rust**, l'application garantit une exécution ultra-rapide, une empreinte mémoire minimale et une préservation absolue de la qualité visuelle perceptible.

---

## Sommaire

1. [Fonctionnalités Principales](#-fonctionnalités-principales)
2. [Règles Métier & Quotas Matériels](#-règles-métier--quotas-matériels)
3. [Architecture Technique & Stack](#-architecture-technique--stack)
4. [Aperçu de l'Interface (4 Onglets)](#-aperçu-de-linterface-4-onglets)
5. [Prérequis Système](#-prérequis-système)
6. [Installation & Lancement en Développement](#-installation--lancement-en-développement)
7. [Architecture du Projet](#-architecture-du-projet)
8. [Configuration du Sidecar FFmpeg](#-configuration-du-sidecar-ffmpeg)
9. [Tests & Qualité du Code](#-tests--qualité-du-code)
10. [Packaging & Distribution Multi-OS](#-packaging--distribution-multi-os)
11. [Feuille de Route (Roadmap)](#-feuille-de-route-roadmap)

---

## 🚀 Fonctionnalités Principales

- **Optimisation d'Images par Recherche Dichotomique :**
  - Ajustement automatique et dynamique du facteur de qualité ($Q \in [65, 92]$) afin de ramener des images lourdes ($\sim 15\text{ Mo}$) dans la fourchette cible stricte de **800 Ko à 2 Mo** (gain moyen de $85\%$ à $95\%$).
  - Prise en charge native des formats **JPG / JPEG**, **PNG** et **WebP**.
  - **Format de sortie au choix** : JPEG (défaut, seul format conservant l'EXIF), WebP avec perte (libwebp), PNG (sans perte, sinon palette 256 couleurs) ou identique à la source. L'extension du fichier produit correspond toujours à son contenu réel.
  - Garde-fou : à format identique, le fichier livré n'est jamais plus lourd que l'original.
  - Traitement parallélisé sur tous les cœurs du processeur grâce à `rayon`.
  - Option de conservation des métadonnées EXIF (date, orientation, appareil photo).

- **Transcodage Vidéo Séquentiel Anti-Surcharge :**
  - Prise en charge des conteneurs vidéo courants (**MP4**, **MOV**, **MKV**, **AVI**).
  - Traitement strictement séquentiel (une vidéo à la fois) piloté de manière asynchrone via `tokio::process`.
  - **Codec au choix** : H.264 (compatibilité maximale), H.265/HEVC (~40 % plus léger) ou VP9 ; **conteneur au choix** : MP4, MKV, MOV, WebM ou identique à la source. Les codecs absents du FFmpeg embarqué sont détectés et désactivés.
  - **CRF plafonné** : le CRF (18 à 32) fixe la qualité plancher, et un plafond de débit calculé depuis la source garantit une sortie ≤ *N* % de l'original (50 % par défaut). Le CRF seul vise une qualité, pas une taille : une vidéo déjà compressée (HEVC de smartphone) ré-encodée au CRF 26 ne perdait que quelques pourcents.
  - Réduction de résolution optionnelle (4K, 1440p, 1080p, 720p, 480p), compatible paysage et portrait.

- **Expérience Utilisateur Temps Réel :**
  - Import fluide par **glisser-déposer (Drag & Drop)** ou explorateur natif.
  - Suivi d'avancement fichier par fichier avec barres de progression synchronisées via événements IPC Tauri (`compression-progress`).
  - Monitoring des ressources système en direct (taux d'usage CPU et consommation RAM).

- **Gestion des Sorties Non-Destructive :**
  - Aucune modification ni écrasement des fichiers originaux.
  - 3 modes de sauvegarde : copie dans un dossier d'export dédié, ajout d'un suffixe `_opt`, ou création d'un sous-dossier horodaté.
  - Empaquetage du lot compressé dans une **archive ZIP unique** téléchargeable via sélecteur système natif.

---

## ⚖️ Règles Métier & Quotas Matériels

| Paramètre | Règle Photos (Images) | Règle Vidéos |
| :--- | :--- | :--- |
| **Plafond par lot** | **50 images maximum** | **20 vidéos maximum** (Plafond strict anti-saturation) |
| **Contrôle de dépassement** | Alerte modale bloquante UI + rejet backend | Alerte modale bloquante immédiate à l'import de la 21ᵉ vidéo |
| **Gabarit cible en sortie** | **800 Ko à 2 Mo** par image | ≤ 50 % de l'original par défaut (plafond réglable de 10 à 100 %), qualité plancher CRF 18-32 |
| **Taux de compression visé** | Réduction de **$85\%$ à $95\%$** sur fichiers HD ($\sim 15\text{ Mo}$) | Réduction sans dégradation de fluidité ni de netteté |
| **Mode d'exécution** | Multi-threadé CPU en parallèle (`rayon`) | File d'attente FIFO séquentielle (1 flux à la fois) |

---

## 🛠 Architecture Technique & Stack

L'application repose sur le modèle multi-processus sécurisé de **Tauri v2** :

```text
┌────────────────────────────────────────────────────────┐
│             FRONTEND (WebView Native de l'OS)          │
│          React.js 19 · TypeScript · Design System      │
└───────────────────────────┬────────────────────────────┘
                            │
               Pont IPC (@tauri-apps/api/core)
               [Passage exclusif par PathBuf]
               [Zéro transit binaire en Base64]
                            │
┌───────────────────────────┴────────────────────────────┐
│                  BACKEND CORE (Rust)                   │
│                                                        │
│  ┌───────────────────┐  ┌───────────────────────────┐  │
│  │   image_engine    │  │       video_engine        │  │
│  │  Rayon / MozJPEG  │  │  FFmpeg Sidecar / Tokio   │  │
│  │   Dichotomie Q    │  │    File séquentielle      │  │
│  └───────────────────┘  └───────────────────────────┘  │
│  ┌───────────────────┐  ┌───────────────────────────┐  │
│  │   export_engine   │  │      metrics_engine       │  │
│  │  std::fs / Zip    │  │      sysinfo CPU/RAM      │  │
│  └───────────────────┘  └───────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

- **Frontend :**
  - **Framework :** React 19 + TypeScript
  - **Bundler :** Vite 8
  - **Design System :** Reproduction fidèle du prototype interactif officiel (`ds/go-compressor/components/bundle.css`, typographie Montserrat, tokens de surface et d'accents azur/cyan).
- **Backend & Système :**
  - **Core :** Rust (édition 2024)
  - **Traitement d'images :** `image`, `mozjpeg`, `webp`, `rayon`
  - **Traitement vidéo :** Binaire `ffmpeg` embarqué en mode sidecar piloté par `tokio::process`
  - **Archivage :** Crate `zip`
  - **Télémétrie :** Crate `sysinfo`

---

## 🖥 Aperçu de l'Interface (Navigation Épurée)

L'interface utilisateur est centrée sur l'efficacité opérationnelle avec une barre de navigation épurée à **2 onglets principaux** et des flux de travail séparés :

1. **Tableau de bord (Vue d'action complète) :**
   - **Deux sous-onglets médias dédiés :**
     - *Photos & Images :* Importation réservée aux images (`JPG`, `PNG`, `WebP`) jusqu'à 50 photos (~15 Mo), calibration cible (800 Ko – 2 Mo) et file d'attente détaillée.
     - *Vidéos :* Importation réservée aux vidéos (`MP4`, `MOV`, `MKV`, `AVI`) plafonnée à 20 vidéos maximum avec transcodage séquentiel anti-saturation.
   - **Gestion de l'export intégrée :** Sélection immédiate du dossier de destination avec bouton *Parcourir…*.
   - **Rapport de performance & Téléchargement :** Affichage des KPI (*Fichiers traités*, *Gain moyen*, *Poids économisé*) et bouton direct de **Téléchargement du lot (.ZIP)**.
   - **Supervision temps réel :** Jauge de progression individuelle par fichier, boutons de pause/lancement et indicateurs télémétriques CPU/RAM.

2. **Paramètres (Réglages avancés) :**
   - Réglages fins des moteurs Rust : curseurs $Q_{\min}$, $Q_{\max}$, nombre de threads `rayon`, banc interactif visualisant les étapes de la recherche dichotomique.
   - Curseur de qualité vidéo (CRF 18-32, échelle H.264 convertie pour H.265 et VP9).
   - Choix du comportement d'écriture non-destructive (copie, suffixe `_opt`, sous-dossier horodaté) et conservation des métadonnées EXIF.

---

## 📋 Prérequis Système

### Prérequis Généraux
- **Node.js :** Version 20.x ou supérieure (v24 recommandée)
- **Gestionnaire de paquets :** `npm` 10+
- **Rust :** Version stable 1.80+ avec `cargo`

### Dépendances Système par OS

#### Linux (Debian / Ubuntu)
```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

#### Windows 10/11
- **Visual Studio C++ Build Tools** (avec le composant « Développement Desktop en C++ »).
- **Microsoft Edge WebView2 Runtime** (intégré par défaut sur Windows 11).

#### macOS (Intel & Apple Silicon)
- Outils de ligne de commande Xcode :
```bash
xcode-select --install
```

---

## ⚡ Installation & Lancement en Développement

### 1. Cloner et Installer les Dépendances Frontend
```bash
# Se placer dans le répertoire du projet
cd go-compressor

# Installer les dépendances Node.js
npm install
```

### 2. Vérifier la Chaîne de Compilation Rust
```bash
cd src-tauri
cargo check
cd ..
```

### 3. Lancer l'Application en Mode Développement
Cette commande démarre le serveur de développement Vite et la fenêtre native Tauri avec rechargement à chaud (Hot Reload) :
```bash
npm run tauri dev
```

---

## 📁 Architecture du Projet

```text
go-compressor/
├── .github/
│   └── workflows/
│       └── release.yml          # Pipeline CI/CD de release multiplateforme
├── public/                      # Ressources statiques publiques
├── reference/                   # Références officielles de conception
│   ├── Cahier des Charges.pdf   # Cahier des charges fonctionnel et technique v1.1
│   ├── sp_cifications_...md     # Spécifications détaillées et User Stories
│   └── GO COMPRESSOR — .../     # Prototype interactif HTML/CSS autonome
├── src/                         # Frontend React 19 & TypeScript
│   ├── assets/                  # Logos, icônes et illustrations
│   ├── components/              # Composants React modulaires
│   │   ├── common/              # Boutons, badges, jauges, modales
│   │   ├── layout/              # Header, navigation par onglets
│   │   └── views/               # Vues : Dashboard, Fichiers, Export, Paramètres
│   ├── hooks/                   # Hooks personnalisés (gestion IPC, batch store)
│   ├── styles/                  # Variables CSS et design tokens du prototype
│   ├── types/                   # Définitions TypeScript (IPC, quotas, médias)
│   ├── utils/                   # Fonctions utilitaires (formatage octets, ratios)
│   ├── App.tsx                  # Composant racine orchestrant les onglets
│   └── main.tsx                 # Point d'entrée React
├── src-tauri/                   # Backend natif Rust & configuration Tauri v2
│   ├── binaries/                # Binaires FFmpeg par architecture (sidecars)
│   ├── capabilities/            # Permissions et politiques de sécurité v2
│   │   └── default.json
│   ├── icons/                   # Icônes de l'application toutes plateformes
│   ├── src/
│   │   ├── main.rs              # Point d'entrée de l'exécutable Rust
│   │   ├── lib.rs               # Initialisation Tauri, routeur de commandes IPC
│   │   ├── image_engine.rs      # Moteur dichotomique et parallélisme Rayon
│   │   ├── video_engine.rs      # Moteur FFmpeg et file d'attente séquentielle
│   │   ├── export_engine.rs     # Écriture non-destructive et archive ZIP
│   │   └── metrics_engine.rs    # Sonde télémétrique CPU / RAM
│   ├── Cargo.toml               # Dépendances et métadonnées de build Rust
│   └── tauri.conf.json          # Configuration générale de Tauri v2
├── ROADMAP.md                   # Feuille de route complète et plan des 8 phases
├── README.md                    # Ce document
├── package.json                 # Scripts et dépendances npm
├── tsconfig.json                # Configuration stricte TypeScript
└── vite.config.ts               # Configuration Vite
```

---

## 🎬 Configuration du Sidecar FFmpeg

Pour permettre le transcodage vidéo sans nécessiter l'installation préalable de FFmpeg sur le poste de l'utilisateur final, l'application utilise la fonctionnalité **Sidecar** de Tauri.

Les exécutables statiques doivent être placés dans le dossier `src-tauri/binaries/` avec les suffixes de triplet cible exacts :

| Plateforme Cible | Nom du Binaire Attendu |
| :--- | :--- |
| **Windows x86_64** | `ffmpeg-x86_64-pc-windows-msvc.exe` |
| **Linux x86_64** | `ffmpeg-x86_64-unknown-linux-gnu` |
| **macOS Intel** | `ffmpeg-x86_64-apple-darwin` |
| **macOS Apple Silicon** | `ffmpeg-aarch64-apple-darwin` |

Déclaration dans `src-tauri/tauri.conf.json` :
```json
{
  "bundle": {
    "externalBin": [
      "binaries/ffmpeg"
    ]
  }
}
```

---

## 🧪 Tests & Qualité du Code

Le projet applique les **100 Règles de Code Professionnel** garantissant zéro régression, lisibilité et testabilité.

### Lancer les Tests Frontend (TypeScript / React)
```bash
npm run test
```

### Lancer les Tests Backend (Rust)
```bash
cd src-tauri
cargo test
cd ..
```

### Vérification du Formatage et Linting
```bash
# Vérification du code Rust (zéro warning toléré)
cargo clippy --all-targets -- -D warnings

# Vérification du typage TypeScript
npm run build
```

---

## 📦 Packaging & Distribution Multi-OS

La compilation et la création des installateurs finaux sont entièrement automatisées via le pipeline GitHub Actions (`.github/workflows/release.yml`) à chaque création de tag de version (ex. `v1.0.0`).

### Formats d'Installateurs Générés :

1. **Windows 10/11 (Standard) :** Installateur exécutable `.exe` via **NSIS** intégrant la vérification et l'installation silencieuse du bootstrapper Microsoft Edge WebView2.
2. **Windows 10/11 (Entreprise / GPO) :** Paquet de déploiement d'entreprise `.msi` via **WiX Toolset v3**.
3. **Linux (Debian / Ubuntu) :** Paquet `.deb` généré via **dpkg** avec déclaration automatique des dépendances GTK et WebKit.
4. **macOS (Intel & Apple Silicon) :** Image disque universelle `.dmg` prête à l'emploi.

### Générer le Bundle en Local sur votre OS Actuel :
```bash
npm run tauri build
```
Les artefacts générés se trouveront dans le dossier `src-tauri/target/release/bundle/`.

---

## 🗺 Feuille de Route (Roadmap)

Le plan de développement détaillé est consigné dans le fichier [ROADMAP.md](./ROADMAP.md) à la racine du projet. Il décompose le cycle de vie du projet en 8 phases séquentielles :

- **Phase 1 :** Socle Technique, Outillage & Dépendances
- **Phase 2 :** Design System & Interface Utilisateur (Reproduction Fidèle du Prototype)
- **Phase 3 :** Infrastructure IPC & Gestion d'État Frontend
- **Phase 4 :** Moteur de Compression d'Images Rust (`image_engine.rs`)
- **Phase 5 :** Moteur de Transcodage Vidéo Rust & Sidecar FFmpeg (`video_engine.rs`)
- **Phase 6 :** Moteurs d'Export, Archivage ZIP & Gestion des Paramètres (`export_engine.rs`)
- **Phase 7 :** Métriques Système Temps Réel, Alertes & Résilience
- **Phase 8 :** Recette Globale, Tests Automatisés & Pipeline CI/CD Multi-OS

---

## 📄 Licence & Propriété

Projet développé selon les spécifications fonctionnelles et techniques de **GO COMPRESSOR** (OptiMedia Desktop v1.1). Tous droits réservés.
