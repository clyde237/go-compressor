# Spécifications Fonctionnelles & Techniques — GO COMPRESSOR

**Application Desktop Cross-Platform d'Optimisation et de Compression Multimédia par Lots**  
* **Stack Technique :** Tauri v2, React.js $18+$ (TypeScript), Rust  
* **Systèmes Cibles :** Windows (`.exe`, `.msi`), Linux Debian/Ubuntu (`.deb`), macOS Apple Silicon & Intel (`.dmg`)  
* **Version de référence :** v1.1  

---

## 1. Interface Utilisateur & Parcours (UI/UX)

L'interface principale est conçue pour être directe, épurée et centrée sur l'action :

### 1.1 Navigation Principale à Deux Onglets
* **Onglet Photos :** Espace dédié à l'importation et au traitement par lots des images haute définition.
* **Onglet Vidéos :** Espace dédié à l'importation et au transcodage des fichiers vidéo.

### 1.2 Importation des Médias
* **Bouton « Importer » & Dropzone :** Sélection de fichiers via l'explorateur système natif ou par glisser-déposer (Drag & Drop).
* **Filtrage des formats :**
  * *Photos :* Support natif des formats `JPG`, `PNG` et `WebP`.
  * *Vidéos :* Support des formats vidéo courants (ex. `MP4`, `MOV`, `MKV`, `AVI`) pris en charge par FFmpeg.
* **Validation des quotas en amont :** Contrôle immédiat du nombre de fichiers importés avec affichage d'une alerte UI bloquante en cas de dépassement.

### 1.3 Déclenchement et Suivi de la Compression
* **Bouton « Compresser » :** S'affiche de manière proéminente dès qu'au moins un fichier valide est importé dans l'onglet actif.
* **Jauge de Progression Circulaire :** Affichage visuel en temps réel du pourcentage de progression ($0\%$ à $100\%$) alimenté par les événements IPC du backend Rust (`listen('compression-progress')`).
* **Rapport d'Optimisation Avant / Après :** Affichage pour chaque lot de la taille initiale, de la taille finale après compression et du gain obtenu en pourcentage.

### 1.4 Téléchargement & Export en Archive ZIP
* **Bouton « Télécharger » :** Apparaît une fois le traitement du lot terminé à $100\%$.
* **Empaquetage `.zip` automatique :** Regroupement de l'ensemble des fichiers compressés du lot au sein d'une archive `.zip` unique (ou écriture directe non-destructive selon le choix de l'utilisateur).
* **Sélection native du répertoire de sauvegarde :** Ouverture de la boîte de dialogue système (`@tauri-apps/plugin-dialog`) au moment du clic sur « Télécharger » pour choisir l'emplacement exact d'enregistrement.

---

## 2. Règles Métier & Cibles de Compression

Les règles suivantes synthétisent les exigences des User Stories et les garde-fous matériels du Cahier des Charges :

| Paramètre | Règle Photos (Images) | Règle Vidéos |
| :--- | :--- | :--- |
| **Quota par lot (User Story)** | Maximum **50 photos** par lot | Jusqu'à **50 vidéos** souhaitées côté UI (voir garde-fou matériel ci-dessous) |
| **Garde-fou Matériel (CDC Rust)** | Traitement parallélisé (jusqu'à 50 fichiers de $\sim 15\text{ Mo}$) | Plafond strict configurable (`MAX_VIDEOS = 20`) pour éviter toute saturation RAM / CPU |
| **Taux de compression visé** | Entre **$70\%$ et $85\%$** (jusqu'à $85\% - 95\%$ sur fichiers RAW/HD de $15\text{ Mo}$) | Réduction optimale sans perte visuelle via encodage à facteur de qualité constant |
| **Poids cible en sortie** | Calibré entre **$800\text{ Ko}$ et $2\text{ Mo}$** par image | Déterminé dynamiquement selon la durée et la résolution (`CRF 23-28`) |
| **Qualité visuelle** | Sans dégradation visuelle notable (Lossless perceptuel) | Conservation de la fluidité et de la netteté d'origine |

> **Note d'implémentation sur le quota vidéo :** Le contrat IPC Rust définit par défaut `const MAX_VIDEOS: usize = 20;` avec rejet immédiat de la $21^{\text{e}}$ vidéo pour protéger le processeur et la mémoire vive. Si le quota de 50 vidéos de la User Story est retenu, le traitement s'effectue par **file d'attente strictement séquentielle** afin de ne jamais saturer la machine.

---

## 3. Backlog Unifié des User Stories

| ID & Priorité | Récit Utilisateur (User Story) | Critères d'Acceptation (DoD) | Implémentation Technique (React / Rust) |
| :--- | :--- | :--- | :--- |
| **US-01**<br>Haute ($P_0$) | **En tant qu'** utilisateur,<br>**Je veux** une interface simple avec deux onglets (*Photos* et *Vidéos*) et un bouton *Importer*,<br>**Afin de** séparer clairement mes flux de travail et charger mes médias facilement. | - Bascule fluide entre les 2 onglets.<br>- Bouton *Importer* + zone Drag & Drop fonctionnels.<br>- Affichage immédiat du bouton *Compresser* après l'import. | Composants React.js + TypeScript, gestion d'état par onglet et sélection de fichiers via `@tauri-apps/plugin-dialog`. |
| **US-02**<br>Haute ($P_0$) | **En tant qu'** utilisateur,<br>**Je veux** importer jusqu'à 50 photos ($\sim 15\text{ Mo}$ en moyenne) et les compresser sans perte de qualité visible,<br>**Afin d'** obtenir une réduction de $70\%$ à $95\%$ (cible entre $800\text{ Ko}$ et $2\text{ Mo}$). | - Support `JPG`, `PNG`, `WebP`.<br>- Poids final garanti entre $800\text{ Ko}$ et $2\text{ Mo}$.<br>- Ratio de compression compris entre $70\%$ et $95\%$ selon la taille initiale. | Moteur `/src-tauri/src/image_engine.rs` : Crates Rust `image`, `mozjpeg`, `webp` parallélisés via `rayon` + recherche dichotomique ($Q=65..92$). |
| **US-03**<br>Haute ($P_0$) | **En tant qu'** utilisateur,<br>**Je veux** importer un lot de vidéos (plafonné à 20 ou 50 selon le profil matériel),<br>**Afin de** compresser mes vidéos sans faire planter ni ralentir mon ordinateur. | - Contrôle bloquant au-delà de la limite autorisée avec alerte UI.<br>- File d'attente séquentielle pour maîtriser la consommation CPU/RAM. | Moteur `/src-tauri/src/video_engine.rs` : Sidecar Rust `ffmpeg` exécuté de façon asynchrone via `tokio::process` (`CRF 23-28`). |
| **US-04**<br>Haute ($P_0$) | **En tant qu'** utilisateur,<br>**Je veux** visualiser l'avancement via une **jauge circulaire** avec pourcentage,<br>**Afin de** connaître précisément l'état de la compression en temps réel. | - Jauge circulaire animée de $0\%$ à $100\%$.<br>- Mise à jour fluide en temps réel à chaque étape d'encodage.<br>- Affichage du gain en $\%$. | Émission d'événements IPC depuis Rust (`emit('compression-progress')`) et écoute temps réel côté React (`listen`). |
| **US-05**<br>Moyenne ($P_1$) | **En tant qu'** utilisateur,<br>**Je veux** cliquer sur un bouton *Télécharger* pour récupérer mes fichiers compressés dans une archive `.zip` et choisir le dossier de sauvegarde,<br>**Afin de** ranger et partager facilement mes médias optimisés. | - Bouton *Télécharger* actif en fin de traitement.<br>- Boîte de dialogue native pour choisir le dossier de destination.<br>- Génération d'un fichier `.zip` intègre et non-destructif. | Plugin `@tauri-apps/plugin-dialog` (mode `save`) + écriture système via `std::fs` et compression d'archive via la crate Rust `zip`. |

---

## 4. Architecture Technique & Moteurs de Traitement (Backend Rust)

### 4.1 Modèle Multi-Processus & Pont IPC Sécurisé
* **Rendu Léger :** Utilisation de la WebView native de l'OS (WebView2 sur Windows, WebKitGTK sur Linux, WebKit sur macOS) au lieu d'embarquer Chromium.
* **Passage par Chemins Système Uniquement :** Pour garantir des performances maximales et éviter toute saturation mémoire, les fichiers lourds ($\sim 15\text{ Mo}$ ou vidéos) ne transitent jamais en `base64` dans le pont IPC JSON. Seuls leurs chemins absolus (`PathBuf`) sont transmis de React vers la commande Rust `#[tauri::command] process_media_batch`.

### 4.2 Moteur de Compression d'Images (`image_engine.rs`)
* **Multi-threading CPU :** Traitement simultané du lot d'images sur tous les cœurs disponibles grâce à la crate `rayon`.
* **Calibration Dynamique du Poids ($800\text{ Ko} \rightarrow 2\text{ Mo}$) :**
  * Définition des constantes strictes :
    $$\text{TARGET\_MIN\_BYTES} = 800 \times 1024$$
    $$\text{TARGET\_MAX\_BYTES} = 2 \times 1024 \times 1024$$
  * Application d'un redimensionnement adaptatif couplé à une **recherche dichotomique** sur le facteur de qualité ($Q \in [65, 92]$) jusqu'à atteindre le ratio de compression optimal ($70\%$ à $95\%$) sans dégradation visuelle perceptible.

### 4.3 Moteur de Transcodage Vidéo (`video_engine.rs`)
* **Binaire Embarqué (Sidecar Tauri) :** Exécution autonome de `ffmpeg` sans nécessiter d'installation tierce de la part de l'utilisateur.
* **Exécution Asynchrone & Thermique :** Pilotage non-bloquant via `tokio::process` en file séquentielle avec encodage à facteur de qualité constant (`CRF 23-28`) pour préserver l'équilibre entre taux de compression ($70\% - 85\%$), qualité d'image et charge processeur.

### 4.4 Module d'Archivage & Export (`export_engine.rs`)
* **Écriture Non-Destructive :** Les fichiers originaux ne sont jamais écrasés.
* **Création de l'Archive `.zip` :** Assemblage en flux tendu des fichiers optimisés dans une archive `.zip` déposée à l'emplacement exact sélectionné par l'utilisateur lors du clic sur **Télécharger**.

---

## 5. Distribution Multiplateforme & Pipeline CI/CD

L'application est compilée et publiée automatiquement via **GitHub Actions** (`tauri-apps/tauri-action`) à chaque création d'un tag de version (`v*`) :

1. **Windows 10/11 (Standard) :** Installateur `.exe` via *NSIS* (avec vérification et installation automatique de Microsoft Edge WebView2 Runtime en mode silencieux).
2. **Windows 10/11 (Entreprise GPO) :** Installateur `.msi` via *WiX Toolset v3*.
3. **Linux (Debian / Ubuntu) :** Paquet `.deb` via *dpkg* intégrant la déclaration automatique des dépendances (`libwebkit2gtk-4.1`, `libayatana-appindicator3`, `librsvg2`).
4. **macOS (Apple Silicon & Intel) :** Image disque universelle `.dmg` (`aarch64` & `x86_64`) prête à être glissée dans `/Applications`.

---

## 6. Critères de Validation & Recette (QA)

* **Qualité du Code :** $0$ erreur au lint (`cargo clippy` côté Rust, `eslint` côté React/TypeScript) et $100\%$ des tests unitaires passants (`cargo test` et `vitest`).
* **Recette Métier Photos :** Import d'un lot de 50 images de $\sim 15\text{ Mo}$ ; vérification que $100\%$ des fichiers compressés respectent le taux de réduction ($70\%$ à $95\%$) et la cible de poids ($800\text{ Ko}$ à $2\text{ Mo}$).
* **Recette Métier Vidéos :** Vérification du blocage immédiat au-delà du quota maximal configuré et stabilité de la RAM/CPU durant le transcodage séquentiel.
* **Recette UI & Export :** Fonctionnement fluide de la jauge circulaire de $0\%$ à $100\%$, ouverture du sélecteur de dossier natif au clic sur *Télécharger* et intégrité de l'archive `.zip` générée sur Windows, Linux et macOS.