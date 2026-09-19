export type Locale = 'zh-CN' | 'en';

export const DEFAULT_LOCALE: Locale = 'zh-CN';

export const SUPPORTED_LOCALES: Locale[] = ['zh-CN', 'en'];

/**
 * Translation table shape. Every key the UI uses must be present in
 * every locale; missing keys trip the `t()` runtime warning so the
 * translator picks them up before shipping.
 */
export interface Messages {
  app: {
    title: string;
    subtitle: string;
    nav: {
      repos: string;
      vault: string;
      settings: string;
    };
    actions: {
      retry: string;
      copy: string;
      copied: string;
      save: string;
      cancel: string;
      delete: string;
      restore: string;
      back: string;
      open: string;
      close: string;
      submit: string;
    };
    states: {
      loading: string;
      empty: string;
      error: string;
      networkError: string;
      serverError: string;
    };
    common: {
      yes: string;
      no: string;
      on: string;
      off: string;
      bytes: string;
      version: string;
      versions: string;
      head: string;
      headSha: string;
      headRef: string;
      refCount: string;
      lastUpdated: string;
      path: string;
      sha256: string;
      changeNote: string;
      createdAt: string;
      size: string;
      delta: string;
      base: string;
      headVer: string;
    };
  };
  home: {
    title: string;
    description: string;
    repoCount: (n: number) => string;
  };
  repo: {
    cloneUrl: string;
    cloneHint: string;
    refs: string;
    log: string;
    noRefs: string;
    noLog: string;
    refName: string;
    refSha: string;
    logSha: string;
    logSubject: string;
    notFound: string;
  };
  vault: {
    title: string;
    description: string;
    keyCount: (n: number) => string;
    keyName: string;
    byteLen: string;
    versionCount: string;
    currentValue: string;
    hideValue: string;
    showValue: string;
    timeline: string;
    noVersions: string;
    diff: {
      title: string;
      pickBase: string;
      pickHead: string;
      objectChanged: (yes: boolean) => string;
      fileSizeDelta: (n: number) => string;
      run: string;
      baseGreater: string;
    };
    restore: {
      title: string;
      pickVersion: string;
      confirmHint: string;
      success: (newVer: number) => string;
      error: string;
    };
    delete: {
      confirm: string;
      success: string;
    };
    notFound: string;
  };
  settings: {
    title: string;
    description: string;
    gitgitPassword: string;
    gitgitPasswordHint: string;
    saveSuccess: string;
    backendStatus: string;
    backendOnline: string;
    backendOffline: string;
    backendVersion: string;
  };
  errors: {
    title: string;
    boundary: string;
    reload: string;
    notFoundTitle: string;
    notFoundDescription: string;
    goHome: string;
    unauthenticated: string;
  };
}

export type MessagesByLocale = Record<Locale, Messages>;