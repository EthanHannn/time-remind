import {
  defineConfig,
  presetAttributify,
  presetIcons,
  presetUno,
  transformerDirectives,
} from 'unocss'

export default defineConfig({
  presets: [
    presetUno(),
    presetAttributify(),
    presetIcons({
      scale: 1.2,
      warn: true,
    }),
  ],
  theme: {
    colors: {
      brand: {
        primary: '#0ea5e9',
        success: '#10b981',
        warning: '#f59e0b',
        danger: '#ef4444',
      },
    },
    borderRadius: {
      card: '12px',
      overlay: '16px',
    },
    boxShadow: {
      soft: '0 4px 20px -2px rgba(0, 0, 0, 0.05), 0 2px 10px -1px rgba(0, 0, 0, 0.03)',
      glass: '0 8px 32px 0 rgba(31, 38, 135, 0.07)',
    },
  },
  shortcuts: {
    'flex-center': 'flex items-center justify-center',
    'card-base': 'bg-white/80 dark:bg-zinc-900/80 backdrop-blur-md rounded-card shadow-soft border border-zinc-200/50 dark:border-zinc-800/50',
    'btn-action': 'px-4 py-2 rounded-lg transition-all active:scale-95 disabled:opacity-50 disabled:pointer-events-none',
    'btn-primary': 'btn-action bg-brand-primary text-white hover:bg-brand-primary/90 shadow-lg shadow-brand-primary/20',
    'text-secondary': 'text-zinc-500 dark:text-zinc-400 text-sm',
  },
  transformers: [
    transformerDirectives(),
  ],
})
