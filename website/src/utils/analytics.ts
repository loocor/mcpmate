import ReactGA from 'react-ga4';

const TRACKING_ID = 'G-ZV7DQ1XHEN';

/**
 * initialize Google Analytics
 */
export const initGA = (): void => {
  ReactGA.initialize(TRACKING_ID, {
    testMode: process.env.NODE_ENV === 'test',
    gtagOptions: {
      debug_mode: process.env.NODE_ENV === 'development'
    }
  });
};

/**
 * track page view
 * @param path page path
 */
export const trackPageView = (path: string): void => {
  ReactGA.send({ hitType: 'pageview', page: path });
};

/**
 * track event
 * @param category event category
 * @param action event action
 * @param label event label (optional)
 * @param value event value (optional)
 */
export const trackEvent = (
  category: string,
  action: string,
  label?: string,
  value?: number
): void => {
  ReactGA.event({
    category,
    action,
    label,
    value,
  });
};

/**
 * MCPMate events tracking
 */
export const trackMCPMateEvents = {
  // navigation events
  navClick: (section: string) => trackEvent('Navigation', 'click', section),

  // waitlist events
  waitlistSubmit: () => trackEvent('Waitlist', 'submit', 'Join Waitlist'),
  waitlistSuccess: () => trackEvent('Waitlist', 'success', 'Waitlist Joined'),
  waitlistError: (error: string) => trackEvent('Waitlist', 'error', error),

  // contact form events
  contactSubmit: () => trackEvent('Contact', 'submit', 'Contact Form'),
  contactSuccess: () => trackEvent('Contact', 'success', 'Contact Form Submitted'),
  contactError: (error: string) => trackEvent('Contact', 'error', error),

  // external link click events
  externalLinkClick: (destination: string) => trackEvent('ExternalLink', 'click', destination),

  // feature view events
  featureView: (feature: string) => trackEvent('Feature', 'view', feature),

  // theme toggle events
  themeToggle: (theme: string) => trackEvent('Theme', 'toggle', theme),

  // language change events
  languageChange: (language: string) => trackEvent('Language', 'change', language),

  // download events
  downloadClick: (platform: string) => trackEvent('Download', 'click', platform),
};
