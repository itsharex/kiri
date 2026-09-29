// Lightweight red click ripple drawn over the recording region. This window is not
// excluded from capture, so enabled ripples appear in the exported video.

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

interface ClickEvent {
  x: number;
  y: number;
}

interface Ripple {
  id: number;
  x: number;
  y: number;
  startedAt: number;
}

const RED = "rgb(216, 35, 50)";

let nextId = 1;

export function RippleWindow() {
  const [ripples, setRipples] = useState<Ripple[]>([]);
  const [now, setNow] = useState(performance.now());

  useEffect(() => {
    const unlisten = listen<ClickEvent>("ripple-click", (event) => {
      const { x, y } = event.payload;
      setRipples((current) => [
        ...current.slice(-8),
        { id: nextId++, x, y, startedAt: performance.now() },
      ]);
    });
    const timer = setInterval(() => setNow(performance.now()), 50);
    return () => {
      void unlisten.then((fn) => fn());
      clearInterval(timer);
    };
  }, []);

  const visible = ripples.filter((ripple) => now - ripple.startedAt < 460);

  return (
    <div style={{ position: "fixed", inset: 0, background: "transparent", overflow: "hidden" }}>
      {visible.map((ripple) => {
        const t = now - ripple.startedAt;
        // Scale eases outward while the ring fades in and then out.
        const scaleAt = (from: number, to: number, time: number) => {
          const v = Math.min(time / 0.68, 1);
          return from + (to - from) * (1 - Math.pow(1 - v, 3));
        };
        const opacityAt = (peak: number, time: number) => {
          if (time >= 1) return 0;
          if (time < 0.12) return peak * (time / 0.12);
          if (time < 0.68) return peak;
          return peak * 0.82 * (1 - (time - 0.68) / 0.32);
        };
        return (
          <div key={ripple.id} style={{ position: "absolute", left: 0, top: 0 }}>
            {/* One thin ring marks the click without obscuring the recording. */}
            <Ellipse
              x={ripple.x}
              y={ripple.y}
              width={32}
              scale={scaleAt(0.55, 1.0, t / 460)}
              opacity={opacityAt(0.78, t / 460)}
              stroke={RED}
              strokeWidth={2.5}
            />
            {/* A small center pinpoints the click as the ring expands. */}
            <Ellipse
              x={ripple.x}
              y={ripple.y}
              width={4}
              scale={scaleAt(0.8, 1.0, t / 240)}
              opacity={opacityAt(0.78, t / 240)}
              fill={RED}
            />
          </div>
        );
      })}
    </div>
  );
}

function Ellipse(props: {
  x: number;
  y: number;
  width: number;
  scale: number;
  opacity: number;
  fill?: string;
  stroke?: string;
  strokeWidth?: number;
}) {
  const { x, y, width, scale, opacity, fill, stroke, strokeWidth } = props;
  return (
    <div
      style={{
        position: "absolute",
        left: x - (width / 2) * scale,
        top: y - (width / 2) * scale,
        width: width * scale,
        height: width * scale,
        borderRadius: "50%",
        background: fill ?? "transparent",
        border: stroke ? `${strokeWidth ?? 1}px solid ${stroke}` : "none",
        boxSizing: "border-box",
        opacity,
        transform: "translateZ(0)",
      }}
    />
  );
}
