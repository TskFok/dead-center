import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "./App";
import { OverlayApp } from "./OverlayApp";
import { tauriBridge } from "./shared/bridge";
import "./global.css";

const isOverlay = new URLSearchParams(window.location.search).get("view") === "overlay";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {isOverlay ? <OverlayApp bridge={tauriBridge} /> : <App />}
  </StrictMode>,
);
