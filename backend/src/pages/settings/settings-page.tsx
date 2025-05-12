import React from 'react';
import { useAppStore } from '../../lib/store';
import { Button } from '../../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card';
import { Switch } from '../../components/ui/switch';
import { Moon, Sun, Cpu, Wand2 } from 'lucide-react';

export function SettingsPage() {
  const { theme, setTheme, uiMode, setUiMode } = useAppStore();
  
  return (
    <div className="space-y-6">
      <h2 className="text-3xl font-bold tracking-tight">Settings</h2>
      
      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Appearance</CardTitle>
            <CardDescription>
              Customize the appearance of the MCPMate dashboard
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <h3 className="text-base font-medium">Theme</h3>
                  <p className="text-sm text-slate-500">
                    Choose between light and dark mode
                  </p>
                </div>
                <div className="flex gap-2">
                  <Button
                    variant={theme === 'light' ? 'default' : 'outline'}
                    size="sm"
                    onClick={() => setTheme('light')}
                    className="w-24"
                  >
                    <Sun className="mr-2 h-4 w-4" />
                    Light
                  </Button>
                  <Button
                    variant={theme === 'dark' ? 'default' : 'outline'}
                    size="sm"
                    onClick={() => setTheme('dark')}
                    className="w-24"
                  >
                    <Moon className="mr-2 h-4 w-4" />
                    Dark
                  </Button>
                </div>
              </div>
              
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <h3 className="text-base font-medium">System Preference</h3>
                  <p className="text-sm text-slate-500">
                    Follow your system's theme preference
                  </p>
                </div>
                <Switch 
                  checked={theme === 'system'} 
                  onCheckedChange={(checked) => setTheme(checked ? 'system' : 'light')}
                />
              </div>
            </div>
          </CardContent>
        </Card>
        
        <Card>
          <CardHeader>
            <CardTitle>Interface Mode</CardTitle>
            <CardDescription>
              Choose between simplified or advanced interface options
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="space-y-6">
              <div className="grid grid-cols-1 gap-4">
                <div 
                  className={`cursor-pointer rounded-lg border p-4 ${
                    uiMode === 'wizard' 
                      ? 'border-blue-500 bg-blue-50 dark:border-blue-400 dark:bg-blue-950/50' 
                      : 'border-slate-200 hover:border-slate-300 dark:border-slate-800 dark:hover:border-slate-700'
                  }`}
                  onClick={() => setUiMode('wizard')}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-2">
                      <Wand2 className="h-5 w-5" />
                      <h3 className="font-medium">Wizard Mode</h3>
                    </div>
                    <Switch checked={uiMode === 'wizard'} />
                  </div>
                  <p className="mt-2 text-sm text-slate-500">
                    Simplified interface with step-by-step guidance. Recommended for new users.
                  </p>
                </div>
                
                <div 
                  className={`cursor-pointer rounded-lg border p-4 ${
                    uiMode === 'expert' 
                      ? 'border-blue-500 bg-blue-50 dark:border-blue-400 dark:bg-blue-950/50' 
                      : 'border-slate-200 hover:border-slate-300 dark:border-slate-800 dark:hover:border-slate-700'
                  }`}
                  onClick={() => setUiMode('expert')}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-2">
                      <Cpu className="h-5 w-5" />
                      <h3 className="font-medium">Expert Mode</h3>
                    </div>
                    <Switch checked={uiMode === 'expert'} />
                  </div>
                  <p className="mt-2 text-sm text-slate-500">
                    Advanced interface with all options and features. Recommended for experienced users.
                  </p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}