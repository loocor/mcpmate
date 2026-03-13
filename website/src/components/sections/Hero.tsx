import { ArrowRight, ChevronLeft, ChevronRight } from 'lucide-react';
import { useLanguage } from '../LanguageProvider';
import { getIntroPoster, getIntroVideoUrl } from '../../utils/media';
import Button from '../ui/Button';
import { useEffect, useState } from 'react';

const Hero = () => {
  useEffect(() => {
    const style = document.createElement('style');
    style.innerHTML = `
      @keyframes fadeIn {
        from { opacity: 0.3; }
        to { opacity: 1; }
      }
      
      @keyframes fadeOut {
        from { opacity: 1; }
        to { opacity: 0.3; }
      }
      
      .fade-in {
        animation: fadeIn 0.3s ease-in-out forwards;
      }
      
      .fade-out {
        animation: fadeOut 0.3s ease-in-out forwards;
      }
    `;
    document.head.appendChild(style);

    return () => {
      document.head.removeChild(style);
    };
  }, []);

  const carouselItems = [
    {
      id: 'intro',
      title: 'Intro',
      video: {
        src: getIntroVideoUrl(),
        posterLight: getIntroPoster('light') || '/ui-dashboard-light.jpg',
        posterDark: getIntroPoster('dark') || '/ui-dashboard-dark.jpg',
      },
      images: {
        light: '/ui-dashboard-light.jpg',
        dark: '/ui-dashboard-dark.jpg',
      }
    },
    {
      id: 'clientapps',
      title: 'Client Apps',
      images: {
        light: '/ui-clientapps-light.jpg',
        dark: '/ui-clientapps-dark.jpg'
      }
    }
  ];

  const [activeIndex, setActiveIndex] = useState(0);
  const [isDarkMode, setIsDarkMode] = useState(false);

  const [fadeState, setFadeState] = useState("visible");

  useEffect(() => {
    const interval = setInterval(() => {
      handleSlideChange((activeIndex + 1) % carouselItems.length);
    }, 5000);
    return () => clearInterval(interval);
  }, [activeIndex, carouselItems.length]);

  const handleSlideChange = (newIndex: number) => {
    setFadeState("fade-out");
    setTimeout(() => {
      setActiveIndex(newIndex);
      setFadeState("fade-in");
    }, 300);
    setTimeout(() => {
      setFadeState("visible");
    }, 600);
  };

  useEffect(() => {
    const darkModeMediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    setIsDarkMode(darkModeMediaQuery.matches);

    const handler = (e: MediaQueryListEvent) => setIsDarkMode(e.matches);
    darkModeMediaQuery.addEventListener('change', handler);
    return () => darkModeMediaQuery.removeEventListener('change', handler);
  }, []);

  const nextSlide = () => {
    handleSlideChange((activeIndex + 1) % carouselItems.length);
  };

  const prevSlide = () => {
    handleSlideChange((activeIndex - 1 + carouselItems.length) % carouselItems.length);
  };


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
              <Button size="lg" onClick={() => scrollToSection('download')}>
                <span>{t('hero.cta.download') || 'Download Preview'}</span>
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
              <div className="relative overflow-hidden rounded-xl shadow-2xl w-full md:max-w-[120%] md:-mr-8">
                {(() => {
                  const item = carouselItems[activeIndex] as {
                    images?: { light: string; dark: string };
                    video?: { src?: string | null; posterLight?: string | null; posterDark?: string | null };
                  };
                  const videoSrc = item.video?.src || undefined;
                  const poster = isDarkMode ? item.video?.posterDark : item.video?.posterLight;
                  if (videoSrc) {
                    return (
                      <video
                        key={`video-${activeIndex}`}
                        className={`w-full h-auto object-cover transition-opacity duration-300 ${fadeState === 'fade-out' ? 'opacity-30' : fadeState === 'fade-in' ? 'opacity-90' : 'opacity-100'} carousel-intro-video`}
                        src={videoSrc}
                        poster={poster || undefined}
                        muted
                        autoPlay
                        playsInline
                        loop
                      />
                    );
                  }
                  return (
                    <img
                      src={isDarkMode ? carouselItems[activeIndex].images.dark : carouselItems[activeIndex].images.light}
                      alt={carouselItems[activeIndex].title}
                      className={`w-full h-auto object-cover transition-opacity duration-300 ${fadeState === 'fade-out' ? 'opacity-30' : fadeState === 'fade-in' ? 'opacity-90' : 'opacity-100'}`}
                    />
                  );
                })()}
              
                <div className="absolute inset-0 flex items-center justify-between p-2 group">
                  <button
                    onClick={prevSlide}
                    className="p-1 rounded-full bg-white/25 dark:bg-slate-800/25 text-slate-800 dark:text-white hover:bg-white/70 dark:hover:bg-slate-800/70 group-hover:bg-white/70 dark:group-hover:bg-slate-800/70 transition-all duration-300"
                    aria-label="Previous slide"
                  >
                    <ChevronLeft className="h-5 w-5" />
                  </button>
                  <button
                    onClick={nextSlide}
                    className="p-1 rounded-full bg-white/25 dark:bg-slate-800/25 text-slate-800 dark:text-white hover:bg-white/70 dark:hover:bg-slate-800/70 group-hover:bg-white/70 dark:group-hover:bg-slate-800/70 transition-all duration-300"
                    aria-label="Next slide"
                  >
                    <ChevronRight className="h-5 w-5" />
                  </button>
                </div>
              </div>

              <div className="flex justify-center mt-4 gap-2">
                {carouselItems.map((item, index) => (
                  <button
                    key={item.id}
                    onClick={() => handleSlideChange(index)}
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
