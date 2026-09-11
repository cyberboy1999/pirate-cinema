export {};

declare global {
  interface Window { pirateCinema?:{choosePlayer:()=>Promise<string|null>}; }
}
