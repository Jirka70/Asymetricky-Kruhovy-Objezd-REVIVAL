import { PageIntro } from "@/components/shell";
import RegionDashboard from "@/components/region-dashboard";
export default function RegionPage() {
  return (
    <main id="main" className="page region-page">
      <PageIntro />
      <RegionDashboard />
    </main>
  );
}
