// Turns a scan on screen (an SVG) into a PNG to copy or save, with all its text
// stamped in. Ultrasound.svelte draws with plain SVG attributes so nothing is
// lost here (an SVG drawn as a picture can't see the page's CSS).

export async function svgToPng(svg: SVGSVGElement, scale = 3): Promise<Blob> {
  const copy = svg.cloneNode(true) as SVGSVGElement;
  const w = Number(svg.getAttribute("width")) || svg.viewBox.baseVal.width;
  const h = Number(svg.getAttribute("height")) || svg.viewBox.baseVal.height;
  copy.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  copy.setAttribute("width", String(w));
  copy.setAttribute("height", String(h));
  const url = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)], { type: "image/svg+xml" }));
  try {
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error("Couldn't draw the scan."));
      img.src = url;
    });
    const canvas = document.createElement("canvas");
    canvas.width = w * scale;
    canvas.height = h * scale;
    const c = canvas.getContext("2d")!;
    c.drawImage(img, 0, 0, canvas.width, canvas.height);
    return await new Promise<Blob>((resolve, reject) => canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("Couldn't make the picture."))), "image/png"));
  } finally {
    URL.revokeObjectURL(url);
  }
}
