import { defineConfig } from '@marlinjai/clearify';

export default defineConfig({
  name: 'SteelSeries Sync',
  // Clearify 2 generates llms.txt by default and then requires siteUrl.
  // Kept off so the docs output stays as it was before the upgrade.
  generateLlmsTxt: false,
  sections: [
    { label: 'Docs', docsDir: './docs/public' },
    { label: 'Internal', docsDir: './docs/internal', basePath: '/internal', draft: true },
  ],
  theme: {
    primaryColor: '#FF5200',
    mode: 'auto',
  },
});
