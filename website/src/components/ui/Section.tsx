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
  subtitleClassName?: string;
  snap?: boolean;
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
  subtitleClassName,
  snap = false,
}: SectionProps) => {
  const sectionClass = `py-20 md:py-24 ${snap ? "snap-section" : ""} ${className}`;
  const contentClass = fullWidth ? 'w-full' : 'container mx-auto px-4 md:px-6';
  const headerClass = `mb-8 md:mb-10 ${centered ? 'text-center' : ''}`;
  const headingClass = `${titleClassName ?? 'text-3xl md:text-4xl'} font-bold tracking-tight mb-4`;
  const subtitleAlignmentClass = centered ? 'mx-auto' : '';
  const subtitleToneClass = subtitleClassName ?? 'section-muted';
  const subtitleClass = `text-lg max-w-3xl leading-relaxed ${subtitleAlignmentClass} ${subtitleToneClass}`;

  return (
    <section id={id} className={sectionClass}>
      <div className={contentClass} data-marketing-scroll-content={id ? '' : undefined}>
        {(title || subtitle) && (
          <div className={headerClass}>
            {title && (
              <h2 className={headingClass}>{title}</h2>
            )}
            {subtitle && (
              <p className={subtitleClass}>
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
