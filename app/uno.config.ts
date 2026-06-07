import { defineConfig, presetUno, presetIcons } from 'unocss';

export default defineConfig({
  presets: [
    presetUno({ dark: 'class' }),
    presetIcons({
      scale: 1.2,
      collections: {}
    })
  ],
  theme: {
    colors: {
      primary: {
        DEFAULT: '#aed581',
        light: '#c5e1a5',
        transparent: 'rgba(174, 213, 129, 0.05)',
        transparent2: 'rgba(174, 213, 129, 0.1)',
        transparent3: 'rgba(174, 213, 129, 0.15)',
        green: '#2aa876',
        greenTransparent: 'rgba(42, 168, 118, 0.4)',
        err: '#f36262',
        errTransparent: 'rgba(243, 98, 98, 0.4)',
        warn: '#eea14a',
        info: '#57a0f3',
        bgOk: '#023502',
        bgErr: '#380505'
      },
      bg: '#222',
      board: {
        light: '#ddd',
        dark: '#555'
      }
    },
    borderRadius: {
      DEFAULT: '0.4rem'
    }
  },
  shortcuts: {
    btn: 'px-4 py-2 rounded font-semibold transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed',
    'btn-primary':
      'btn text-primary bg-primary-transparent hover:bg-[rgba(174,213,129,0.15)]',
    'btn-secondary':
      'btn text-white hover:bg-primary-transparent bg-transparent border border-white/10',
    card: 'rounded border border-white/10 bg-white/[0.03] p-3',
    'card-hover': 'card hover:bg-white/5 hover:border-white/20 cursor-pointer',
    'card-accent': 'rounded border border-primary bg-primary-transparent p-3',
    dropdown:
      'absolute rounded border border-white/15 bg-[#1a1a2e] p-1.5 shadow-xl z-50'
  }
});
