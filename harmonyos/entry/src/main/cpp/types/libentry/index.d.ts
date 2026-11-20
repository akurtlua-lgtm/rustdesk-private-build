export const initialize: (home: string, files: string) => number;
export const onEvent: (handler: (event: string) => void) => number;
export const version: () => string;
export const connect: (peer: string, password: string, relay: boolean) => string;
export const login: (session: string, password: string, user: string, osPassword: string, remember: boolean) => number;
export const twoFactor: (session: string, code: string, trust: boolean) => number;
export const close: (session: string) => number;
export const mouse: (session: string, mask: number, x: number, y: number, modifiers: number) => number;
export const key: (session: string, value: string, modifiers: number) => number;
export const text: (session: string, value: string) => number;
export const viewOnly: (session: string, enabled: boolean) => number;
export const account: (operation: string, payload: string, token: string) => Promise<string>;
export interface SharingStatus { phase: number; errorCode: number; deviceId: string; }
export const sharingSupported: () => boolean;
export const sharingPassword: (password: string) => number;
export const sharingStart: (width: number, height: number) => number;
export const sharingStop: () => Promise<number>;
export const sharingStatus: () => SharingStatus;
