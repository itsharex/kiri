export interface TextFrame {
  x: number;
  y: number;
  width: number;
  height: number;
}

type MeasureText = (text: string) => number;

export const TEXT_TAB_SIZE: number;
export function textLineRuns(text: string, measureText: MeasureText): {
  runs: { text: string; x: number }[];
  width: number;
};
export function textEditorInsets(uiScale?: number): { x: number; y: number };

export function layoutTextLines(
  text: string,
  maxWidth: number,
  measureText: MeasureText,
): string[];

export function fitTextEditorFrame(options: {
  text: string;
  fontSize: number;
  x: number;
  y: number;
  maxWidth: number;
  boundsWidth: number;
  boundsHeight: number;
  measureText: MeasureText;
  uiScale?: number;
}): TextFrame;
