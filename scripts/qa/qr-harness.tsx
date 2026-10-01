// An isolated frontend harness. It never invokes native Kiri commands.
import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { QrResults, QrFavorites, QrModal } from "../../src/qr/QrResults";
import { setLanguage } from "../../src/i18n";
import "../../src/styles/design-system.css";

const params = new URLSearchParams(location.search);
setLanguage(params.get("language") === "en" ? "en" : "zh-Hans");
const scenario = params.get("scenario") ?? "multi";
const base = "/src-tauri/tests/fixtures/qr/";
const meta = await fetch(base + (scenario === "single" ? "url" : scenario === "favorites" ? "multi" : scenario) + ".json").then(r => r.json());
const image = base + (scenario === "single" ? "url" : scenario) + ".png";
function ModalHarness() {
  const [open, setOpen] = useState(true);
  return open ? <QrModal scan={{...meta, requestId:"fixture-request", imageUrl:image}} onClose={() => setOpen(false)}/> : <p role="status">Mock capture canceled</p>;
}
createRoot(document.getElementById("root")!).render(<div className="library-root kiri-canvas-surface" style={{height:"100vh",boxSizing:"border-box",padding:28}}>
  <header className="text-dialog__header"><div><h2>{scenario === "favorites" ? "二维码收藏" : "二维码"}</h2><p>隔离前端测试 · 公开生成的测试图片 · 模拟 IPC</p></div><a href="?scenario=favorites">查看收藏测试</a></header>
  {params.get("presentation") === "modal" ? <ModalHarness/> : scenario === "favorites" ? <div style={{height:"calc(100vh - 112px)",display:"flex"}}><QrFavorites/></div> : <QrResults scan={{...meta,requestId:"fixture-request",imageUrl:image}}/>}
  <output id="qa-actions" style={{display:"block",marginTop:20,fontSize:12}}/>
</div>);
