import { ArrowRight } from 'lucide-react';
import { useLanguage } from '../LanguageProvider';
import Button from '../ui/Button';
import BrowserFrame from '../ui/BrowserFrame';
import { useEffect, useState } from 'react';

const Hero = () => {
  const carouselItems = [
    {
      id: 'servers',
      title: 'Server Management',
      image: '/hero-servers.png',
      url: 'localhost:5173/servers'
    },
    {
      id: 'client',
      title: 'Client Configuration',
      image: '/hero-client.png',
      url: 'localhost:5173/clients/cursor'
    },
    {
      id: 'profile',
      title: 'Profile Overview',
      image: '/hero-profile.png',
      url: 'localhost:5173/profiles'
    }
  ];

  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setActiveIndex((prev) => (prev + 1) % carouselItems.length);
    }, 5000);
    return () => clearInterval(interval);
  }, [carouselItems.length]);

  const scrollToSection = (id: string) => {
    const element = document.getElementById(id);
    if (element) {
      const offset = 80;
      const elementPosition = element.getBoundingClientRect().top;
      const offsetPosition = elementPosition + window.pageYOffset - offset;

      window.scrollTo({
        top: offsetPosition,
        behavior: 'smooth'
      });
    }
  };

  const { t } = useLanguage();
  return (
    <div className="pt-24 md:pt-32 pb-16 md:pb-24 px-4 md:px-6">
      <div className="container mx-auto relative z-10">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-8 items-center">
          <div className="flex flex-col items-start space-y-6">
            <div className="inline-flex items-center rounded-full px-3 py-1 text-sm font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300">
              <span className="flex h-2 w-2 rounded-full bg-blue-500 mr-2"></span>
              {t('hero.early_access')}
            </div>

            <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold tracking-tight leading-tight text-slate-900 dark:text-white">
              <span>{t('hero.title')}</span>
              <br />
              <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-600 to-cyan-600 dark:from-blue-400 dark:to-cyan-400">
                {t('hero.subtitle')}
              </span>
            </h1>

            <p className="text-lg md:text-xl text-slate-600 dark:text-slate-400 max-w-lg">
              {t('hero.description')}
            </p>

            <div className="flex flex-col sm:flex-row gap-4 pt-2">
              <Button size="lg" onClick={() => window.open('https://github.com/loocor/mcpmate', '_blank')}>
                <span>{t('hero.cta.download')}</span>
                <ArrowRight className="ml-2 h-5 w-5" />
              </Button>
              <Button variant="outline" size="lg" onClick={() => scrollToSection('features')}>
                {t('hero.cta.learn')}
              </Button>
            </div>

            <div className="grid grid-cols-3 gap-4 pt-6">
              <div className="flex flex-col">
                <span className="text-2xl font-bold text-slate-900 dark:text-white">70%</span>
                <span className="text-sm text-slate-500 dark:text-slate-400">{t('hero.stats.config')}</span>
              </div>
              <div className="flex flex-col">
                <span className="text-2xl font-bold text-slate-900 dark:text-white">50%</span>
                <span className="text-sm text-slate-500 dark:text-slate-400">{t('hero.stats.resource')}</span>
              </div>
              <div className="flex flex-col">
                <span className="text-2xl font-bold text-slate-900 dark:text-white">100%</span>
                <span className="text-sm text-slate-500 dark:text-slate-400">{t('hero.stats.integration')}</span>
              </div>
            </div>
          </div>

          <div className="relative">
            <div className="aspect-square max-w-xl mx-auto md:ml-auto rounded-xl bg-gradient-to-br from-blue-500 to-cyan-500 opacity-10 dark:opacity-20 blur-3xl absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 z-0"></div>
            <div className="relative z-10">
              <BrowserFrame 
                url={carouselItems[activeIndex].url}
                className="w-full md:max-w-[120%] md:-mr-8 transition-all duration-500"
              >
                <img
                  key={carouselItems[activeIndex].id}
                  src={carouselItems[activeIndex].image}
                  alt={carouselItems[activeIndex].title}
                  className="w-full h-auto object-cover"
                />
              </BrowserFrame>

              <div className="flex justify-center mt-4 gap-2">
                {carouselItems.map((item, index) => (
                  <button
                    key={item.id}
                    onClick={() => setActiveIndex(index)}
                    className={`h-2 rounded-full transition-all ${activeIndex === index ? 'w-6 bg-blue-500' : 'w-2 bg-slate-300 dark:bg-slate-600'}`}
                    aria-label={`Go to slide ${index + 1}`}
                  />
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Hero;
