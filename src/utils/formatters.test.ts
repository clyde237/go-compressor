import { describe, it, expect } from 'vitest';
import { formatBytes, calculateGainPercent, isWithinTargetRange } from './formatters';

describe('formatBytes', () => {
  it('doit formater correctement en Ko pour moins de 1 Mo', () => {
    expect(formatBytes(500 * 1024)).toBe('500 Ko');
    expect(formatBytes(800 * 1024)).toBe('800 Ko');
    expect(formatBytes(0)).toBe('0 Ko');
  });

  it('doit formater correctement en Mo pour 1 Mo ou plus', () => {
    expect(formatBytes(1.5 * 1024 * 1024)).toBe('1.5 Mo');
    expect(formatBytes(15.2 * 1024 * 1024)).toBe('15.2 Mo');
    expect(formatBytes(1 * 1024 * 1024)).toBe('1.0 Mo');
  });
});

describe('calculateGainPercent', () => {
  it('doit calculer le gain exact pour une réduction de 15 Mo à 1.5 Mo (~90%)', () => {
    const initial = 15 * 1024 * 1024;
    const final = 1.5 * 1024 * 1024;
    expect(calculateGainPercent(initial, final)).toBe(90);
  });

  it('doit retourner 0 si la taille finale est supérieure ou égale', () => {
    expect(calculateGainPercent(1000, 1200)).toBe(0);
    expect(calculateGainPercent(1000, 1000)).toBe(0);
    expect(calculateGainPercent(0, 500)).toBe(0);
  });
});

describe('isWithinTargetRange', () => {
  it('doit valider les fichiers dans la fourchette 800 Ko - 2 Mo', () => {
    expect(isWithinTargetRange(800 * 1024)).toBe(true);
    expect(isWithinTargetRange(1104 * 1024)).toBe(true);
    expect(isWithinTargetRange(2000 * 1024)).toBe(true);
  });

  it('doit rejeter les fichiers hors fourchette', () => {
    expect(isWithinTargetRange(500 * 1024)).toBe(false);
    expect(isWithinTargetRange(3 * 1024 * 1024)).toBe(false);
  });
});
