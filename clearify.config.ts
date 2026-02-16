import { defineConfig } from '@marlinjai/clearify';

export default defineConfig({
  name: 'SteelSeries Sync',
  sections: [
    { label: 'Docs', docsDir: './docs/public' },
    { label: 'Internal', docsDir: './docs/internal', basePath: '/internal', draft: true },
  ],
  theme: {
    primaryColor: '#FF5200',
    mode: 'auto',
  },
});
