import type { Metadata } from "next";
import "./globals.css";
import "./compact.css";
import "./workspace.css";
import { Header, Footer } from "@/components/shell";

export const metadata: Metadata = {
  title: "Obor na dosah",
  description: "Dostupnost středních škol v Karlovarském kraji.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="cs" data-scroll-behavior="smooth">
      <body>
        <Header />
        {children}
        <Footer />
      </body>
    </html>
  );
}
