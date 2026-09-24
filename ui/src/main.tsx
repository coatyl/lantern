import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { I18nProvider } from "./i18n/I18nProvider";

// Self-hosted fonts: no CDN dependency, works fully offline.
import "@fontsource-variable/inter";           // Inter variable (wght 100-900)
import "@fontsource/jetbrains-mono";           // JetBrains Mono 400

import "./index.css";

// Only English ships, so the locale is fixed until a second one lands.
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <I18nProvider locale="en">
      <App />
    </I18nProvider>
  </React.StrictMode>
);
