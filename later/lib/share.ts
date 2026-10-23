// Turns an on-screen SVG into a PNG (keepsakes, share cards).

import { saveBlob } from "./api";

export async function svgToPng(svg: SVGSVGElement, width: number, height: number): Promise<Blob> {
  const clone = svg.cloneNode(true) as SVGSVGElement;
  clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  clone.setAttribute("width", String(width));
  clone.setAttribute("height", String(height));
  // Resolve theme variables, since an image can't see the page's CSS.
  const styles = getComputedStyle(document.documentElement);
  let text = new XMLSerializer().serializeToString(clone);
  text = text.replace(/var\((--[a-z0-9-]+)\)/gi, (_, name) => styles.getPropertyValue(name).trim() || "#f59ac0");
  const url = URL.createObjectURL(new Blob([text], { type: "image/svg+xml;charset=utf-8" }));
  try {
    const img = new Image();
    await new Promise<void>((res, rej) => {
      img.onload = () => res();
      img.onerror = () => rej(new Error("Could not draw the image"));
      img.src = url;
    });
    const canvas = document.createElement("canvas");
    const scale = 2;
    canvas.width = width * scale;
    canvas.height = height * scale;
    const ctx = canvas.getContext("2d")!;
    ctx.scale(scale, scale);
    ctx.drawImage(img, 0, 0, width, height);
    return await new Promise<Blob>((res, rej) => canvas.toBlob((b) => (b ? res(b) : rej(new Error("Could not make a PNG"))), "image/png"));
  } finally {
    URL.revokeObjectURL(url);
  }
}

export async function saveSvgAsPng(svg: SVGSVGElement, width: number, height: number, name: string) {
  const blob = await svgToPng(svg, width, height);
  return saveBlob(name, blob, "png");
}
