import { ReactNode } from 'react';

interface CardProps {
  children: ReactNode;
  className?: string;
  hoverEffect?: boolean;
}

const Card = ({ children, className = '', hoverEffect = false }: CardProps) => {
  return (
    <div 
      className={`
        bg-white dark:bg-slate-800 
        rounded-2xl overflow-hidden bg-clip-padding isolate shadow-md 
        border border-slate-200 dark:border-slate-700
        ${hoverEffect ? 'transition-all duration-300 hover:shadow-lg hover:scale-[1.02]' : ''}
        ${className}
      `}
    >
      {children}
    </div>
  );
};

export const CardHeader = ({ children, className = '' }: { children: ReactNode; className?: string }) => {
  return (
    <div className={`p-5 ${className}`}>
      {children}
    </div>
  );
};

export const CardContent = ({ children, className = '' }: { children: ReactNode; className?: string }) => {
  return (
    <div className={`p-5 pt-0 ${className}`}>
      {children}
    </div>
  );
};

export const CardFooter = ({ children, className = '' }: { children: ReactNode; className?: string }) => {
  return (
    <div className={`p-5 pt-0 ${className}`}>
      {children}
    </div>
  );
};

export default Card;
