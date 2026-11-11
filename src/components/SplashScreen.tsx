import { useEffect, useState } from "react";
import splash from "../assets/brand/splash-screen.png";

const MIN_SPLASH_MS = 7000;

/** Abertura do app: a arte oficial de splash fica ~7s centralizada em
    meia escala (padrão splash de IDE) enquanto o shell carrega; some por
    fade (ou corte, com movimento reduzido). */
export function SplashScreen() {
  const [phase, setPhase] = useState<"show" | "fade" | "done">("show");

  useEffect(() => {
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const fadeTimer = setTimeout(() => setPhase("fade"), reduced ? 600 : MIN_SPLASH_MS);
    const doneTimer = setTimeout(() => setPhase("done"), (reduced ? 600 : MIN_SPLASH_MS) + (reduced ? 0 : 460));
    return () => { clearTimeout(fadeTimer); clearTimeout(doneTimer); };
  }, []);

  if (phase === "done") return null;
  return <div className={phase === "fade" ? "splash splash--fade" : "splash"} aria-hidden="true">
    <img src={splash} alt="" className="splash-art" />
    <p className="splash-hint">Eureka42 — Universal CLI</p>
  </div>;
}
