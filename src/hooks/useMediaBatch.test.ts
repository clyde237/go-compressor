import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useMediaBatch } from './useMediaBatch';

describe('useMediaBatch Hook', () => {
  it('doit initialiser avec un état vide', () => {
    const { result } = renderHook(() => useMediaBatch());

    expect(result.current.files).toEqual([]);
    expect(result.current.running).toBe(false);
    expect(result.current.hasRun).toBe(false);
    expect(result.current.alert).toBeNull();
  });

  it('doit ajouter des images dans la file si le quota est respecté', () => {
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.addFilesFromPaths(['/images/photo1.jpg', '/images/photo2.png'], 'images');
    });

    expect(result.current.files.length).toBe(2);
    expect(result.current.files[0].name).toBe('photo1.jpg');
    expect(result.current.files[0].kind).toBe('image');
    expect(result.current.alert).toBeNull();
  });

  it('doit bloquer et lever une alerte si le quota de 20 vidéos est dépassé', () => {
    const { result } = renderHook(() => useMediaBatch());

    const twentyOneVideos = Array.from({ length: 21 }, (_, i) => `/videos/clip_${i}.mp4`);

    act(() => {
      result.current.addFilesFromPaths(twentyOneVideos, 'videos');
    });

    expect(result.current.files.length).toBe(0);
    expect(result.current.alert).not.toBeNull();
    expect(result.current.alert?.title).toBe('Quota vidéo dépassé');
    expect(result.current.alert?.body).toContain('20 vidéos maximum');
  });

  it('doit bloquer et lever une alerte si le quota de 50 photos est dépassé', () => {
    const { result } = renderHook(() => useMediaBatch());

    const fiftyOneImages = Array.from({ length: 51 }, (_, i) => `/images/img_${i}.jpg`);

    act(() => {
      result.current.addFilesFromPaths(fiftyOneImages, 'images');
    });

    expect(result.current.files.length).toBe(0);
    expect(result.current.alert).not.toBeNull();
    expect(result.current.alert?.title).toBe('Quota photos dépassé');
    expect(result.current.alert?.body).toContain('50 images maximum');
  });

  it('doit supprimer un élément par son id', () => {
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.addFilesFromPaths(['/images/photo1.jpg'], 'images');
    });

    const fileId = result.current.files[0].id;

    act(() => {
      result.current.removeItem(fileId);
    });

    expect(result.current.files.length).toBe(0);
  });

  it('doit vider la file par type de média', () => {
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.addFilesFromPaths(['/images/photo1.jpg'], 'images');
      result.current.addFilesFromPaths(['/videos/clip1.mp4'], 'videos');
    });

    expect(result.current.files.length).toBe(2);

    act(() => {
      result.current.clearQueue('images');
    });

    expect(result.current.files.length).toBe(1);
    expect(result.current.files[0].kind).toBe('video');
  });
});
