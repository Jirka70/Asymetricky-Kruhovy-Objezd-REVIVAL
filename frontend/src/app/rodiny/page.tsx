import { PageIntro } from "@/components/shell";
import FamilyDashboard from "@/components/family-dashboard";
export default function FamiliesPage() {
  return (
    <main id="main" className="page family-page">
      <PageIntro family />
      <FamilyDashboard />
    </main>
  );
}
