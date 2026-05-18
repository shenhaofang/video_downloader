import QRCode from "qrcode";

export async function createQrDataUrl(text: string): Promise<string> {
  const svg = await QRCode.toString(text, {
    type: "svg",
    margin: 1,
    width: 180,
  });
  return `data:image/svg+xml;charset=UTF-8,${encodeURIComponent(svg)}`;
}
