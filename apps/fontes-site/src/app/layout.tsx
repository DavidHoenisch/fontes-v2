import type { Metadata } from "next";
import localFont from "next/font/local";
import "./globals.css";
const sans = localFont({
  src: [
    { path: "../../public/fonts/IBMPlexSans-Regular.ttf", weight: "400" },
    { path: "../../public/fonts/IBMPlexSans-Medium.ttf", weight: "500 700" },
  ],
  variable: "--font-plex",
});
const serif = localFont({
  src: "../../public/fonts/Lora-Regular.ttf",
  variable: "--font-lora",
});
export const metadata: Metadata = {
  title: "Fontes · Publisher workspace",
  description: "Prepare thoughtful resources for the Fontes library.",
  icons: { icon: "/brand/app-icon-rounded.svg" },
  robots: { index: false, follow: false },
};
export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${sans.variable} ${serif.variable}`}>
      <body>{children}</body>
    </html>
  );
}
