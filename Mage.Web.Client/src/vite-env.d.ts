/// <reference types="vite/client" />

interface ImportMetaEnv {
    readonly VITE_APP_VERSION?: string;
    readonly VITE_APP_NAME?: string;
    readonly VITE_IMAGE_PROXY?: string;
    readonly VITE_MAGE_SERVER_URL?: string;
    readonly VITE_NATIVE?: string;
    readonly VITE_PUBLIC_ORIGIN?: string;
    /** profiling builds only (e2e/perf-live.mjs): the game stores on window, for the autopilot */
    readonly VITE_PERF_HOOKS?: string;
}

interface ImportMeta {
    readonly env: ImportMetaEnv;
}
