import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
export default defineConfig({
  plugins: [react()],
  optimizeDeps: { entries: ["scripts/qa/qr-harness.html"] },
  resolve: { alias: [{find:"../lib/ipc",replacement:fileURLToPath(new URL("./qr-ipc-mock.ts",import.meta.url))}] },
  server: { host:"127.0.0.1",port:5187,strictPort:true },
});
