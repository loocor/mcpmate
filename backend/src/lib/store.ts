import { create } from 'zustand';
import { Theme, UIMode } from './types';

interface AppState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  uiMode: UIMode;
  setUiMode: (mode: UIMode) => void;
  sidebarOpen: boolean;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  theme: 'system',
  setTheme: (theme) => set({ theme }),
  uiMode: 'expert', // Default to expert mode
  setUiMode: (uiMode) => set({ uiMode }),
  sidebarOpen: true,
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarOpen: (open) => set({ sidebarOpen: open }),
}));