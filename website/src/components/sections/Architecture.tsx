import Section from '../ui/Section';
import Card from '../ui/Card';
import { CheckCircle2, ShieldCheck, Smile, Gauge } from 'lucide-react';
import { useLanguage } from '../LanguageProvider';

const Architecture = () => {
  const { t } = useLanguage();
  return (
    <Section
      title={t('arch.title')}
      titleClassName="text-4xl"
      subtitle={t('arch.subtitle')}
      centered
      className="bg-white dark:bg-slate-900 border-t border-slate-200/70 dark:border-slate-800/60"
    >
      <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mb-12">
        <Card hoverEffect className="h-full">
          <div className="p-6">
            <div className="w-12 h-12 flex items-center justify-center rounded-lg bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 mb-4">
              <Gauge size={24} />
            </div>
            <h3 className="text-xl font-semibold mb-4">{t('arch.values.performance.title')}</h3>
            <p className="text-slate-600 dark:text-slate-400 mb-4">{t('arch.values.performance.desc')}</p>
            <div className="space-y-2 mt-4">
              <div className="flex items-start">
                <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5 mr-2 flex-shrink-0" />
                <span className="text-sm text-slate-600 dark:text-slate-400">
                  {t('arch.values.performance.p1')}
                </span>
              </div>
              <div className="flex items-start">
                <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5 mr-2 flex-shrink-0" />
                <span className="text-sm text-slate-600 dark:text-slate-400">
                  {t('arch.values.performance.p2')}
                </span>
              </div>
              <div className="flex items-start">
                <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5 mr-2 flex-shrink-0" />
                <span className="text-sm text-slate-600 dark:text-slate-400">
                  {t('arch.values.performance.p3')}
                </span>
              </div>
              
            </div>
          </div>
        </Card>

        <Card hoverEffect className="h-full">
          <div className="p-6">
            <div className="w-12 h-12 flex items-center justify-center rounded-lg bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 mb-4">
              <Smile size={24} />
            </div>
            <h3 className="text-xl font-semibold mb-4">{t('arch.values.experience.title')}</h3>
            <p className="text-slate-600 dark:text-slate-400 mb-4">{t('arch.values.experience.desc')}</p>
            <div className="space-y-2 mt-4">
              <div className="flex items-start">
                <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5 mr-2 flex-shrink-0" />
                <span className="text-sm text-slate-600 dark:text-slate-400">
                  {t('arch.values.experience.p1')}
                </span>
              </div>
              <div className="flex items-start">
                <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5 mr-2 flex-shrink-0" />
                <span className="text-sm text-slate-600 dark:text-slate-400">
                  {t('arch.values.experience.p2')}
                </span>
              </div>
              <div className="flex items-start">
                <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5 mr-2 flex-shrink-0" />
                <span className="text-sm text-slate-600 dark:text-slate-400">
                  {t('arch.values.experience.p3')}
                </span>
              </div>
              
            </div>
          </div>
        </Card>

        <Card hoverEffect className="h-full">
          <div className="p-6">
            <div className="w-12 h-12 flex items-center justify-center rounded-lg bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 mb-4">
              <ShieldCheck size={24} />
            </div>
            <h3 className="text-xl font-semibold mb-4">{t('arch.values.security.title')}</h3>
            <p className="text-slate-600 dark:text-slate-400 mb-4">{t('arch.values.security.desc')}</p>
            <div className="space-y-2 mt-4">
              <div className="flex items-start">
                <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5 mr-2 flex-shrink-0" />
                <span className="text-sm text-slate-600 dark:text-slate-400">
                  {t('arch.values.security.p1')}
                </span>
              </div>
              <div className="flex items-start">
                <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5 mr-2 flex-shrink-0" />
                <span className="text-sm text-slate-600 dark:text-slate-400">
                  {t('arch.values.security.p2')}
                </span>
              </div>
              <div className="flex items-start">
                <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5 mr-2 flex-shrink-0" />
                <span className="text-sm text-slate-600 dark:text-slate-400">
                  {t('arch.values.security.p3')}
                </span>
              </div>
              
            </div>
          </div>
        </Card>
      </div>
    </Section>
  );
};

export default Architecture;
