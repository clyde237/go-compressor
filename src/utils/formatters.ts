/**
 * Utilitaires de conversion et formatage de taille et de gain
 */

/**
 * Formate un nombre d'octets en unité lisible (Ko ou Mo)
 * Suit scrupuleusement la convention du prototype interactif :
 * Mo < 1 -> "XXX Ko", Mo >= 1 -> "X.X Mo"
 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 Ko';
  const mo = bytes / (1024 * 1024);
  if (mo < 1) {
    return `${Math.round(bytes / 1024)} Ko`;
  }
  return `${mo.toFixed(1)} Mo`;
}

/**
 * Calcule le pourcentage de réduction (gain) entre taille initiale et finale
 */
export function calculateGainPercent(initialBytes: number, finalBytes: number): number {
  if (initialBytes <= 0 || finalBytes <= 0 || finalBytes >= initialBytes) {
    return 0;
  }
  return Math.floor((1 - finalBytes / initialBytes) * 100);
}

/**
 * Vérifie si le fichier est dans la cible 800 Ko – 2 Mo
 */
export function isWithinTargetRange(bytes: number): boolean {
  const minBytes = 780 * 1024; // tolérance basse
  const maxBytes = 2060 * 1024; // tolérance haute
  return bytes >= minBytes && bytes <= maxBytes;
}
