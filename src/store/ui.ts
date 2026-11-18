import { create } from "zustand";

interface UiState {
  captureOpen: boolean;
  openCapture: () => void;
  closeCapture: () => void;
  paletteOpen: boolean;
  openPalette: () => void;
  closePalette: () => void;
}

export const useUi = create<UiState>((set) => ({
  captureOpen: false,
  openCapture: () => set({ captureOpen: true }),
  closeCapture: () => set({ captureOpen: false }),
  paletteOpen: false,
  openPalette: () => set({ paletteOpen: true }),
  closePalette: () => set({ paletteOpen: false }),
}));
