import Section from '../ui/Section';
import Card from '../ui/Card';
import Button from '../ui/Button';
import { Mail, Github } from 'lucide-react';
import { useState, FormEvent } from 'react';
import { useLanguage } from '../LanguageProvider';
import { trackMCPMateEvents } from '../../utils/analytics';

const ContactSection = () => {
  const { t } = useLanguage();
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [message, setMessage] = useState('');
  const [submitted, setSubmitted] = useState(false);
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Formspree endpoint 
  const FORMSPREE_CONTACT_URL = 'https://formspree.io/f/xbloyrel';

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();

    if (!name.trim() || !email.trim() || !message.trim()) {
      const errorMsg = t('contact.error.required');
      setError(errorMsg);
      trackMCPMateEvents.contactError(errorMsg);
      return;
    }

    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      const errorMsg = t('contact.error.email');
      setError(errorMsg);
      trackMCPMateEvents.contactError(errorMsg);
      return;
    }

    // track form submission event
    trackMCPMateEvents.contactSubmit();

    setIsSubmitting(true);
    setError('');

    try {
      // submit to Formspree
      const response = await fetch(FORMSPREE_CONTACT_URL, {
        method: 'POST',
        headers: {
          'Accept': 'application/json',
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          name,
          email,
          message,
          date: new Date().toISOString()
        })
      });

      if (!response.ok) {
        throw new Error('Submission failed');
      }

      // store to local storage (as backup)
      const messages = JSON.parse(localStorage.getItem('mcpmate_messages') || '[]');
      messages.push({
        name,
        email,
        message,
        date: new Date().toISOString()
      });
      localStorage.setItem('mcpmate_messages', JSON.stringify(messages));

      // print to console, for development
      console.log('Contact message:', { name, email, message });

      setSubmitted(true);
      setName('');
      setEmail('');
      setMessage('');

      // track form submission success event
      trackMCPMateEvents.contactSuccess();
    } catch (err) {
      const errorMsg = t('contact.error.submit');
      setError(errorMsg);
      trackMCPMateEvents.contactError(errorMsg);
      console.error('Contact submission error:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Section className="bg-slate-50 dark:bg-slate-800/40 border-t border-slate-200/70 dark:border-slate-700/50">
      <div className="max-w-4xl mx-auto text-center mb-16">
        <h2 className="text-4xl font-bold mb-6">{t('contact.title')}</h2>
        <p className="text-xl text-slate-600 dark:text-slate-400">
          {t('contact.subtitle')}
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-8 max-w-5xl mx-auto">
        <div className="md:col-span-2">
          <Card>
            <div className="p-6">
              <h3 className="text-2xl font-bold mb-6">{t('contact.message')}</h3>

              {submitted ? (
                <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-6 text-center">
                  <h4 className="text-xl font-semibold text-green-800 dark:text-green-300 mb-2">
                    {t('contact.success.title')}
                  </h4>
                  <p className="text-green-700 dark:text-green-400">
                    {t('contact.success.message')}
                  </p>
                </div>
              ) : (
                <form onSubmit={handleSubmit} className="mx-auto">
                  <div className="space-y-4">
                    <div>
                      <label htmlFor="name" className="block text-sm font-medium mb-1">
                        {t('contact.name')}
                      </label>
                      <input
                        type="text"
                        id="name"
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        className="w-full px-4 py-2 rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        placeholder={t('contact.name.placeholder')}
                        required
                      />
                    </div>

                    <div>
                      <label htmlFor="email" className="block text-sm font-medium mb-1">
                        {t('contact.email')}
                      </label>
                      <input
                        type="email"
                        id="email"
                        value={email}
                        onChange={(e) => setEmail(e.target.value)}
                        className="w-full px-4 py-2 rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        placeholder={t('contact.email.placeholder')}
                        required
                      />
                    </div>

                    <div>
                      <label htmlFor="message" className="block text-sm font-medium mb-1">
                        {t('contact.message.label')}
                      </label>
                      <textarea
                        id="message"
                        value={message}
                        onChange={(e) => setMessage(e.target.value)}
                        rows={5}
                        className="w-full px-4 py-2 rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        placeholder={t('contact.message.placeholder')}
                        required
                      ></textarea>
                    </div>

                    {error && (
                      <p className="text-sm text-red-600 dark:text-red-400">
                        {error}
                      </p>
                    )}

                    <Button type="submit" size="lg" disabled={isSubmitting}>
                      {isSubmitting ? '...' : t('contact.send')}
                    </Button>
                  </div>
                </form>
              )}
            </div>
          </Card>
        </div>

        <div>
          <div className="space-y-6">
            <Card hoverEffect>
              <div className="p-6">
                <div className="flex items-start">
                  <div className="mr-4">
                    <div className="w-10 h-10 flex items-center justify-center rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400">
                      <Mail size={20} />
                    </div>
                  </div>
                  <div>
                    <h3 className="text-lg font-semibold mb-1">{t('contact.email.us')}</h3>
                    <p className="text-slate-600 dark:text-slate-400 mb-2">
                      {t('contact.email.desc')}
                    </p>
                    <a
                      href="mailto:loocor@gmail.com"
                      className="text-blue-600 dark:text-blue-400 hover:underline"
                      onClick={() => trackMCPMateEvents.externalLinkClick('mailto:loocor@gmail.com')}
                    >
                      loocor@gmail.com
                    </a>
                  </div>
                </div>
              </div>
            </Card>

            <Card hoverEffect>
              <div className="p-6">
                <div className="flex items-start">
                  <div className="mr-4">
                    <div className="w-10 h-10 flex items-center justify-center rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400">
                      <Github size={20} />
                    </div>
                  </div>
                  <div>
                    <h3 className="text-lg font-semibold mb-1">{t('contact.github')}</h3>
                    <p className="text-slate-600 dark:text-slate-400 mb-2">
                      {t('contact.github.desc')}
                    </p>
                    <a
                      href="https://github.com/loocor/mcpmate"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 dark:text-blue-400 hover:underline"
                      onClick={() => trackMCPMateEvents.externalLinkClick('github.com/loocor/mcpmate')}
                    >
                      github.com/loocor/mcpmate
                    </a>
                  </div>
                </div>
              </div>
            </Card>
          </div>
        </div>
      </div>
    </Section>
  );
};

export default ContactSection;
