import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import {native, NativeApp} from "./native";
import "./styles.css";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {native ? <NativeApp /> : <App />}
  </React.StrictMode>,
);
