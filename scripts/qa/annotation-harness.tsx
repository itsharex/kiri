// Isolated annotation QA: generated source pixels, no native app or user library.
import React, {useCallback, useEffect, useRef, useState} from "react";
import {createRoot} from "react-dom/client";
import AnnotationCanvas, {type AnnotationCanvasHandle} from "../../src/annotation/AnnotationCanvas";
import {DEFAULT_APPEARANCE, type AnnotationMark, type AnnotationDocumentV1, type Tool} from "../../src/annotation/model";
import "../../src/styles/design-system.css";

const source = "data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><rect width="640" height="360" fill="white"/><text x="32" y="48" font-family="sans-serif" font-size="18" fill="#555">Kiri annotation fixture</text></svg>');
const original: AnnotationMark = {kind:"rectangle",id:1,rect:{x:100,y:100,width:140,height:90},color:"cherry",width:4};
const fresh = new URLSearchParams(location.search).has("fresh");
const initialDocument: AnnotationDocumentV1 = {schemaVersion:1,canvas:{width:640,height:360},sourcePixels:{width:640,height:360},marks:fresh?[]:[original]};

function Harness() {
  const canvas=useRef<AnnotationCanvasHandle>(null);
  const [image,setImage]=useState<HTMLImageElement|null>(null);
  const [tool,setTool]=useState<Tool>(fresh?"rectangle":"select");
  const [marks,setMarks]=useState(initialDocument.marks);
  useEffect(()=>{const img=new Image();img.onload=()=>setImage(img);img.src=source;return()=>{img.onload=null;};},[]);
  const frame=useCallback((value:HTMLCanvasElement)=>{Object.assign(window,{__qaCanvas:value});},[]);
  const history=useCallback(()=>{},[]);
  return <div className="library-root kiri-canvas-surface" style={{padding:24,boxSizing:"border-box",height:"100vh"}}>
    <div style={{display:"flex",gap:8,marginBottom:16}}>
      <button className="kiri-button" onClick={()=>setTool("select")}>Select</button>
      <button className="kiri-button" onClick={()=>setTool("rectangle")}>Rectangle</button>
      <button className="kiri-button" onClick={()=>canvas.current?.undo()}>Undo</button>
      <button className="kiri-button" onClick={()=>canvas.current?.redo()}>Redo</button>
    </div>
    {image&&<AnnotationCanvas ref={canvas} image={image} region={{x:0,y:0,width:640,height:360}}
      initialDocument={initialDocument} tool={tool} appearance={DEFAULT_APPEARANCE}
      onHistoryChange={history} onCancel={()=>{}} onFrame={frame} onDocumentChange={setMarks}/>}
    <output id="qa-marks">{JSON.stringify(marks)}</output>
  </div>;
}
createRoot(document.getElementById("root")!).render(<Harness/>);
