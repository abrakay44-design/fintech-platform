import type { Metadata } from "next";
import "./globals.css";
import { Providers } from "./providers";

export const metadata: Metadata = {
  title: "Fintech Swap",
  description: "Non-custodial swap platform",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className="font-sans antialiased bg-gray-950 text-white">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
