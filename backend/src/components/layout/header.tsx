import React from 'react';
import { useLocation } from 'react-router-dom';
import { Moon, Sun, BellRing } from 'lucide-react';
import { Switch } from '../ui/switch';
import { Button } from '../ui/button';
import { useAppStore } from '../../lib/store';
import { Badge } from '../ui/badge';

const ROUTE_TITLES: Record<string, string> = {
  '/': 'Dashboard',
  '/servers': 'Servers',
  '/tools': 'Tools',
  '/system': 'System',
  '/settings': 'Settings',
};

export function Header() {
  const location = useLocation();
  const { theme, setTheme, uiMode, setUiMode, sidebarOpen } = useAppStore();
  
  const toggleTheme = () => {
    setTheme(theme === 'dark' ? 'light' : 'dark');
  };
  
  const toggleMode = () => {
    setUiMode(uiMode === 'expert' ? 'wizard' : 'expert');
  };
  
  const pageTitle = ROUTE_TITLES[location.pathname] || 'Not Found';
  
  return (
    <header className={`fixed top-0 right-0 z-30 flex h-16 items-center border-b border-slate-200 bg-white px-4 dark:border-slate-800 dark:bg-slate-950 ${
      sidebarOpen ? 'left-64' : 'left-16'
    } transition-all duration-300 ease-in-out`}>
      <div className="flex w-full items-center justify-between">
        <h1 className="text-xl font-semibold text-slate-900 dark:text-slate-50">
          {pageTitle}
        </h1>
        
        <div className="flex items-center space-x-4">
          <div className="flex items-center">
            <span className="mr-2 text-sm font-medium text-slate-700 dark:text-slate-400">
              {uiMode === 'expert' ? 'Expert' : 'Wizard'}
            </span>
            <Switch 
              checked={uiMode === 'expert'} 
              onCheckedChange={toggleMode} 
              aria-label="Toggle UI Mode"
            />
          </div>
          
          <div className="flex items-center">
            <Button variant="ghost" size="icon" onClick={toggleTheme} aria-label="Toggle theme">
              {theme === 'dark' ? <Sun size={20} /> : <Moon size={20} />}
            </Button>
          </div>
          
          <div className="flex items-center">
            <Button variant="ghost" size="icon" aria-label="Notifications">
              <div className="relative">
                <BellRing size={20} />
                <Badge className="absolute -top-1 -right-1 h-4 w-4 p-0 flex items-center justify-center" variant="destructive">
                  2
                </Badge>
              </div>
            </Button>
          </div>
        </div>
      </div>
    </header>
  );
}