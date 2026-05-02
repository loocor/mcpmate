import Section from '../ui/Section';
import Card from '../ui/Card';
import { useLanguage } from '../LanguageProvider';

const ValueProposition = () => {
  const { t } = useLanguage();
  return (
    <Section className="border-t border-slate-200/70 dark:border-slate-800/60" id="why">
      <div className="max-w-5xl mx-auto">
        <h2 className="text-4xl font-bold text-center mb-3">{t('value.title')}</h2>
        <p className="text-lg text-slate-600 dark:text-slate-400 text-center mb-10">{t('value.subtitle')}</p>

        <div className="space-y-12">
          {/* Creators */}
          <div className="grid grid-cols-1 md:grid-cols-5 gap-8 items-start">
            <div className="md:col-span-3 order-2 md:order-1">
              <h3 className="text-2xl font-semibold mb-4">{t('value.creators.title')}</h3>
              <ul className="text-slate-600 dark:text-slate-400 list-disc pl-5 space-y-2">
                <li>{t('value.creators.p1')}</li>
                <li>{t('value.creators.p2')}</li>
                <li>{t('value.creators.p3')}</li>
              </ul>
            </div>
            <Card hoverEffect className="md:col-span-2 order-1 md:order-2 p-0 overflow-hidden">
              <div className="aspect-video rounded-lg overflow-hidden bg-slate-100 dark:bg-slate-800">
                <img
                  src="/why-creator-flow.jpg"
                  alt={t('value.creators.diagram')}
                  className="h-full w-full object-cover"
                />
              </div>
            </Card>
          </div>

          {/* Team Leads */}
          <div className="grid grid-cols-1 md:grid-cols-5 gap-8 items-start">
            <Card hoverEffect className="md:col-span-2 p-0 overflow-hidden">
              <div className="aspect-video rounded-lg overflow-hidden bg-slate-100 dark:bg-slate-800">
                <img
                  src="/why-team-consistency.jpg"
                  alt={t('value.managers.diagram')}
                  className="h-full w-full object-cover"
                />
              </div>
            </Card>
            <div className="md:col-span-3">
              <h3 className="text-2xl font-semibold mb-4">{t('value.managers.title')}</h3>
              <ul className="text-slate-600 dark:text-slate-400 list-disc pl-5 space-y-2">
                <li>{t('value.managers.p1')}</li>
                <li>{t('value.managers.p2')}</li>
                <li>{t('value.managers.p3')}</li>
              </ul>
            </div>
          </div>

          {/* Enterprise Owners */}
          <div className="grid grid-cols-1 md:grid-cols-5 gap-8 items-start">
            <div className="md:col-span-3 order-2 md:order-1">
              <h3 className="text-2xl font-semibold mb-4">{t('value.owners.title')}</h3>
              <ul className="text-slate-600 dark:text-slate-400 list-disc pl-5 space-y-2">
                <li>{t('value.owners.p1')}</li>
                <li>{t('value.owners.p2')}</li>
                <li>{t('value.owners.p3')}</li>
              </ul>
            </div>
            <Card hoverEffect className="md:col-span-2 order-1 md:order-2 p-0 overflow-hidden">
              <div className="aspect-video rounded-lg overflow-hidden bg-slate-100 dark:bg-slate-800">
                <img
                  src="/why-enterprise-governance.jpg"
                  alt={t('value.owners.diagram')}
                  className="h-full w-full object-cover"
                />
              </div>
            </Card>
          </div>
        </div>
      </div>
    </Section>
  );
};

export default ValueProposition;
