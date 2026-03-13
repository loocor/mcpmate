import { ReactNode } from 'react';

interface SectionProps {
  title?: string;
  subtitle?: string;
  children: ReactNode;
  className?: string;
  id?: string;
  centered?: boolean;
  fullWidth?: boolean;
  titleClassName?: string;
}

const Section = ({
  title,
  subtitle,
  children,
  className = '',
  id,
  centered = false,
  fullWidth = false,
  titleClassName,
}: SectionProps) => {
  return (
    <section id={id} className={`py-20 md:py-24 ${className}`}>
      <div className={`${fullWidth ? 'w-full' : 'container mx-auto px-4 md:px-6'}`}>
        {(title || subtitle) && (
          <div className={`mb-10 md:mb-12 ${centered ? 'text-center' : ''}`}>
            {title && (
              <h2 className={`${titleClassName ?? 'text-3xl md:text-4xl'} font-bold tracking-tight mb-4`}>{title}</h2>
            )}
            {subtitle && (
              <p className="text-lg text-slate-600 dark:text-slate-400 max-w-3xl mx-auto">
                {subtitle}
              </p>
            )}
          </div>
        )}
        {children}
      </div>
    </section>
  );
};

export default Section;
