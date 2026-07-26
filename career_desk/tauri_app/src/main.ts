import "./shared/styles/tokens.css";
import "./shared/styles/app.css";
import { App } from "./app";
if (import.meta.env.MODE === "e2e") void import("./e2e-bridge").then(({mountE2EBridge})=>mountE2EBridge());
const root = document.querySelector<HTMLElement>("#app");
if (!root) throw new Error("应用挂载节点不存在");
new App(root).mount();
