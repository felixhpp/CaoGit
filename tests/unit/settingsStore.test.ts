/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../src/services/gitApi', () => ({
  GitApi: {
    setWindowTheme: vi.fn().mockResolvedValue({ success: true })
  }
}));

describe('settingsStore', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.resetModules();
  });

  it('preserves nested defaults when importing partial settings', async () => {
    const { settingsStore } = await import('../../src/stores/settingsStore');

    expect(settingsStore.importSettings(JSON.stringify({
      sync: { autoRefreshInterval: 25 }
    }))).toBe(true);

    expect(settingsStore.settings.sync.autoRefreshInterval).toBe(25);
    expect(settingsStore.settings.sync.refreshOnFocus).toBe(true);
    expect(settingsStore.settings.layout.sidebarWidth).toBe(240);
  });

  it('never includes credentials in exported settings', async () => {
    const { settingsStore } = await import('../../src/stores/settingsStore');

    settingsStore.saveSettings({
      proxy: {
        ...settingsStore.settings.proxy,
        username: 'proxy-user',
        password: 'proxy-secret'
      },
      gitPlatforms: {
        ...settingsStore.settings.gitPlatforms,
        github: {
          ...settingsStore.settings.gitPlatforms.github,
          token: 'github-secret'
        }
      },
      githubToken: 'release-secret'
    });

    const exported = settingsStore.exportSettings();
    expect(exported).not.toContain('proxy-secret');
    expect(exported).not.toContain('github-secret');
    expect(exported).not.toContain('release-secret');
  });

  it('removes credentials from persistence when remembering is disabled', async () => {
    const { settingsStore } = await import('../../src/stores/settingsStore');

    settingsStore.saveSettings({
      security: {
        ...settingsStore.settings.security,
        rememberCredentials: false
      },
      githubToken: 'release-secret'
    });

    expect(localStorage.getItem('global_settings')).not.toContain('release-secret');
  });
});
