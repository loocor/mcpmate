import { type ReactNode } from 'react';

interface BrowserFrameProps {
  children: ReactNode;
  url?: string;
  className?: string;
}

const BrowserFrame = ({ children, url = 'localhost:5173', className = '' }: BrowserFrameProps) => {
  return (
    <div className={`rounded-xl overflow-hidden shadow-2xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 ${className}`}>
      <div className="flex items-center gap-2 px-4 py-2 bg-slate-100 dark:bg-slate-900 border-b border-slate-200 dark:border-slate-700">
        <div className="flex items-center gap-1.5">
          <div className="w-3 h-3 rounded-full bg-red-500 hover:bg-red-600 transition-colors cursor-pointer" />
          <div className="w-3 h-3 rounded-full bg-yellow-500 hover:bg-yellow-600 transition-colors cursor-pointer" />
          <div className="w-3 h-3 rounded-full bg-green-500 hover:bg-green-600 transition-colors cursor-pointer" />
        </div>
        
        <div className="flex-1 ml-4">
          <div className="flex items-center gap-2 px-3 py-1 rounded-md bg-slate-200 dark:bg-slate-800 text-xs text-slate-600 dark:text-slate-400 font-mono">
            <svg 
              className="w-3 h-3 text-slate-400" 
              fill="none" 
              stroke="currentColor" 
              viewBox="0 0 24 24"
            >
              <path 
                strokeLinecap="round" 
                strokeLinejoin="round" 
                strokeWidth={2} 
                d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" 
              />
            </svg>
            <span>{url}</span>
          </div>
        </div>
      </div>
      
      <div className="bg-white dark:bg-slate-900">
        {children}
      </div>
    </div>
  );
};

export default BrowserFrame;
