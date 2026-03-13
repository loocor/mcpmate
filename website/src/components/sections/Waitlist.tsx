import { useEffect } from 'react';
import Section from '../ui/Section';
import Button from '../ui/Button';
import { useLanguage } from '../LanguageProvider';
import { useForm, ValidationError } from '@formspree/react';
import { trackMCPMateEvents } from '../../utils/analytics';

const Waitlist = () => {
  const { t } = useLanguage();

  // use Formspree's useForm hook, replace with your actual form ID
  const [state, handleSubmit] = useForm("xqaqbzol");

  // custom form submit handler
  const handleFormSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    trackMCPMateEvents.waitlistSubmit();
    handleSubmit(e);
  };

  // check if form has been successfully submitted
  const submitted = state.succeeded;

  // track form submission success event
  useEffect(() => {
    if (submitted) {
      trackMCPMateEvents.waitlistSuccess();
    }
  }, [submitted]);

  return (
    <Section
      id="waitlist"
      className="bg-gradient-to-b from-blue-50 to-white dark:from-slate-800/50 dark:to-slate-900"
    >
      <div className="max-w-3xl mx-auto text-center">
        <h2 className="text-3xl md:text-4xl font-bold mb-4">
          {t('waitlist.title')}
        </h2>
        <p className="text-lg text-slate-600 dark:text-slate-400 mb-8">
          {t('waitlist.description')}
        </p>

        {submitted ? (
          <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-6 text-center">
            <h3 className="text-xl font-semibold text-green-800 dark:text-green-300 mb-2">
              {t('waitlist.thanks.title')}
            </h3>
            <p className="text-green-700 dark:text-green-400">
              {t('waitlist.thanks.desc')}
            </p>
          </div>
        ) : (
          <form onSubmit={handleFormSubmit} className="mx-auto max-w-md">
            <div className="flex flex-col sm:flex-row gap-3">
              <input
                type="email"
                name="email"
                placeholder={t('waitlist.email')}
                className="flex-1 px-4 py-3 rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                disabled={state.submitting}
                required
              />
              <Button
                type="submit"
                size="lg"
                disabled={state.submitting}
              >
                {state.submitting ? t('waitlist.joining') : t('waitlist.join')}
              </Button>
            </div>
            <ValidationError
              prefix="Email"
              field="email"
              errors={state.errors}
              className="mt-2 text-sm text-red-600 dark:text-red-400"
            />
            <p className="mt-3 text-sm text-slate-500 dark:text-slate-400">
              {t('waitlist.privacy')}
            </p>
          </form>
        )}
      </div>
    </Section>
  );
};

export default Waitlist;