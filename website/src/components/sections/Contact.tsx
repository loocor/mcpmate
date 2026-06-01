import { Mail } from 'lucide-react';
import Section from '../ui/Section';
import { useLanguage } from '../LanguageProvider';
import { trackMCPMateEvents } from '../../utils/analytics';

const ContactSection = () => {
  const { t } = useLanguage();

  return (
    <Section id="contact" className="border-t border-brand-border-subtle py-16 md:py-20">
      <div className="mx-auto max-w-3xl text-center">
        <h2 className="text-3xl md:text-4xl font-bold text-brand-foreground">{t('contact.title')}</h2>
        <p className="mt-4 text-lg section-muted">{t('contact.subtitle')}</p>

        <div className="mt-8 glass-card rounded-2xl p-6 md:p-8">
          <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-xl bg-brand-accent/10 text-brand-accent">
            <Mail size={22} />
          </div>
          <p className="mt-4 text-sm md:text-base section-muted">{t('contact.email.desc')}</p>
          <a
            href="mailto:loocor@gmail.com"
            className="mt-5 inline-flex items-center justify-center rounded-lg bg-brand-accent px-5 py-2.5 text-sm font-semibold text-brand-accent-fg transition-all hover:bg-brand-accent-hover focus:outline-none focus:ring-2 focus:ring-brand-accent focus:ring-offset-2 focus:ring-offset-brand-bg dark:hover:ring-2 dark:hover:ring-white dark:hover:ring-offset-2 dark:hover:ring-offset-brand-bg dark:focus-visible:ring-2 dark:focus-visible:ring-white dark:focus-visible:ring-offset-2 dark:focus-visible:ring-offset-brand-bg"
            onClick={() => trackMCPMateEvents.externalLinkClick('mailto:loocor@gmail.com')}
          >
            {t('contact.email.us')}
          </a>
          <p className="mt-3 text-sm text-brand-muted">loocor@gmail.com</p>
        </div>
      </div>
    </Section>
  );
};

export default ContactSection;
