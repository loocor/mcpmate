import { ReactNode, ButtonHTMLAttributes } from 'react';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'outline' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  children: ReactNode;
  fullWidth?: boolean;
}

const Button = ({
  variant = 'primary',
  size = 'md',
  children,
  fullWidth = false,
  className = '',
  ...props
}: ButtonProps) => {
  const baseClasses = 'inline-flex items-center justify-center rounded-lg font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-brand-accent focus:ring-offset-2 focus:ring-offset-brand-bg';

  const variantClasses = {
    primary: 'bg-brand-accent text-brand-accent-fg hover:bg-brand-accent-hover shadow-card dark:hover:ring-2 dark:hover:ring-white dark:hover:ring-offset-2 dark:hover:ring-offset-brand-bg dark:focus-visible:ring-2 dark:focus-visible:ring-white dark:focus-visible:ring-offset-2 dark:focus-visible:ring-offset-brand-bg',
    secondary: 'bg-brand-overlay text-brand-foreground hover:bg-brand-overlay-hover border border-brand-border-subtle',
    outline: 'bg-transparent border border-brand-border hover:bg-brand-overlay text-brand-foreground',
    ghost: 'bg-transparent hover:bg-brand-overlay text-brand-foreground',
  };

  const sizeClasses = {
    sm: 'text-xs px-3 py-1.5',
    md: 'text-sm px-4 py-2',
    lg: 'text-base px-6 py-2.5',
  };

  const widthClass = fullWidth ? 'w-full' : '';

  return (
    <button
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${widthClass} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};

export default Button;
