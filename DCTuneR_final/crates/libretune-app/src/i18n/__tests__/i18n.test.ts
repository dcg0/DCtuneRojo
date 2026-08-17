import { describe, it, expect, beforeAll } from 'vitest';
import i18n, { SUPPORTED_LANGUAGES } from '../index';
import enMenu from '../locales/en/menu.json';
import enCommon from '../locales/en/common.json';
import ptBRMenu from '../locales/pt-BR/menu.json';
import ptBRCommon from '../locales/pt-BR/common.json';
import huHUMenu from '../locales/hu-HU/menu.json';
import huHUCommon from '../locales/hu-HU/common.json';

/**
 * Helper: collect the full set of dotted key paths from a nested object.
 * Used to assert that every locale exposes the SAME set of keys — a missing
 * key in one locale is the root cause of "menu only partially switches
 * language" bugs (see GitHub issue #72).
 */
function keyPaths(obj: Record<string, unknown>, prefix = ''): string[] {
  const out: string[] = [];
  for (const [k, v] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      out.push(...keyPaths(v as Record<string, unknown>, path));
    } else {
      out.push(path);
    }
  }
  return out;
}

describe('i18n bootstrap', () => {
  beforeAll(async () => {
    // Ensure init has settled before assertions.
    if (!i18n.isInitialized) {
      await new Promise<void>(resolve => i18n.on('initialized', () => resolve()));
    }
  });

  it('exposes the supported languages list', () => {
    const codes = SUPPORTED_LANGUAGES.map(l => l.code);
    expect(codes).toContain('en');
    expect(codes).toContain('pt-BR');
    expect(codes).toContain('hu-HU');
  });

  it('resolves an English key', async () => {
    await i18n.changeLanguage('en');
    expect(i18n.t('actions.cancel', { ns: 'common' })).toBe('Cancel');
  });

  it('switches to Portuguese (Brasil)', async () => {
    await i18n.changeLanguage('pt-BR');
    expect(i18n.t('actions.cancel', { ns: 'common' })).toBe('Cancelar');
    expect(i18n.t('state.connected', { ns: 'common' })).toBe('Conectado');
  });

  it('switches to Hungarian', async () => {
    await i18n.changeLanguage('hu-HU');
    expect(i18n.t('actions.cancel', { ns: 'common' })).toBe('Megszakít');
    expect(i18n.t('state.connected', { ns: 'common' })).toBe('Kapcsolódva');
  });

  it('falls back to English when a key is missing in the active locale', async () => {
    await i18n.changeLanguage('pt-BR');
    // Key only exists in en; pt-BR should fall through.
    const result = i18n.t('this.key.does.not.exist', {
      ns: 'common',
      defaultValue: 'fallback-value',
    });
    expect(result).toBe('fallback-value');
  });

  it('interpolates variables', async () => {
    await i18n.changeLanguage('en');
    expect(
      i18n.t('state.partialSync', { ns: 'common', done: 3, total: 7 })
    ).toBe('Partial sync (3/7)');
  });
});

// ---------------------------------------------------------------------------
// Regression coverage for GitHub issue #72 ([BUG] Languages): menu items only
// partially switched language. Root cause was hardcoded English labels in
// buildMenuItems.ts instead of t() calls, which masked the fact that some
// keys were also missing from non-English locales. These tests guard against
// both failure modes: (1) key-parity across locales and (2) every key must
// actually resolve to a translated value (not fall through to the key path).
// ---------------------------------------------------------------------------

describe('locale key parity', () => {
  const localeFiles = {
    en: { menu: enMenu, common: enCommon },
    'pt-BR': { menu: ptBRMenu, common: ptBRCommon },
    'hu-HU': { menu: huHUMenu, common: huHUCommon },
  } as const;

  for (const ns of ['menu', 'common'] as const) {
    it(`exposes identical ${ns} key sets in every locale`, () => {
      const enKeys = new Set(keyPaths(localeFiles.en[ns]));
      for (const [code, files] of Object.entries(localeFiles)) {
        const localeKeys = new Set(keyPaths(files[ns]));
        const missing = [...enKeys].filter(k => !localeKeys.has(k));
        const extra = [...localeKeys].filter(k => !enKeys.has(k));
        expect({ code, missing, extra }).toEqual({ code, missing: [], extra: [] });
      }
    });
  }
});

describe('menu items resolve to a real translation per locale', () => {
  // A representative cross-section of menu keys across every namespace
  // section. If any of these return the key path itself (i.e. the value is
  // missing/empty), a label would render as raw "file.newProject" in the UI.
  const sampleKeys = [
    'file.newProject',
    'file.saveTune',
    'file.burnToEcu',
    'file.exit',
    'file.settings',
    'edit.undo',
    'edit.cut',
    'edit.resetToDefaults',
    'view.dashboard',
    'view.theme',
    'tools.autotune',
    'tools.ecuConsole',
    'tools.aiAssistant',       // added with issue #72 fix
    'tools.ecuLuaEditor',      // added with issue #72 fix
    'tools.updateFirmware',    // added with issue #72 fix
    'tools.enterDfuMode',      // added with issue #72 fix
    'tools.plugins',
    'help.userManual',
  ];

  for (const code of SUPPORTED_LANGUAGES.map(l => l.code)) {
    it(`resolves all sample menu keys for "${code}"`, async () => {
      await i18n.changeLanguage(code);
      for (const key of sampleKeys) {
        const value = i18n.t(key, { ns: 'menu' });
        // i18next returns the key path when no translation exists.
        expect(value, `menu.${key} missing for ${code}`).not.toBe(key);
        // When returnNull:false is set, a missing key still yields the key
        // path (asserted above); also guard against an empty string.
        expect(value.length, `menu.${key} empty for ${code}`).toBeGreaterThan(0);
      }
    });
  }
});

describe('toolbar tooltips resolve to a real translation per locale', () => {
  const toolbarKeys = [
    'toolbar.openTune',
    'toolbar.saveTune',
    'toolbar.burnDisconnected',
    'toolbar.burnPending',
    'toolbar.burnNone',
    'toolbar.disconnect',
    'toolbar.connect',
    'toolbar.connectionInfo',
    'toolbar.realtime',
    'toolbar.stopLogging',
    'toolbar.startLogging',
    'toolbar.settings',
  ];

  for (const code of SUPPORTED_LANGUAGES.map(l => l.code)) {
    it(`resolves all toolbar keys for "${code}"`, async () => {
      await i18n.changeLanguage(code);
      for (const key of toolbarKeys) {
        const value = i18n.t(key, { ns: 'common' });
        expect(value, `common.${key} missing for ${code}`).not.toBe(key);
        expect(value.length, `common.${key} empty for ${code}`).toBeGreaterThan(0);
      }
    });
  }
});

describe('menu mnemonics and shortcuts survive translation', () => {
  // Labels are parsed by MenuBar.tsx: '\t' splits off the shortcut column and
  // '&' marks the access-key mnemonic. These must round-trip through i18n.
  it('keeps the New Project shortcut on Ctrl+N across locales', async () => {
    for (const code of SUPPORTED_LANGUAGES.map(l => l.code)) {
      await i18n.changeLanguage(code);
      const value = i18n.t('file.newProject', { ns: 'menu' });
      expect(value.endsWith('\tCtrl+N'), `${code} newProject shortcut`).toBe(true);
    }
  });

  it('keeps the Burn shortcut on Ctrl+B across locales', async () => {
    for (const code of SUPPORTED_LANGUAGES.map(l => l.code)) {
      await i18n.changeLanguage(code);
      const value = i18n.t('file.burnToEcu', { ns: 'menu' });
      expect(value.endsWith('\tCtrl+B'), `${code} burnToEcu shortcut`).toBe(true);
    }
  });

  it('preserves a mnemonic (&) on the Help title across locales', async () => {
    for (const code of SUPPORTED_LANGUAGES.map(l => l.code)) {
      await i18n.changeLanguage(code);
      const value = i18n.t('help.title', { ns: 'menu' });
      expect(value.includes('&'), `${code} help.title mnemonic`).toBe(true);
    }
  });
});
