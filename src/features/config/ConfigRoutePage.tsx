import { useNavigate } from "@tanstack/react-router";

import { ConfigScreen } from "@/features/config/ConfigScreen";
import { getSectionRoute, type ConfigSectionId } from "@/features/config/sections";

type ConfigRoutePageProps = {
  sectionId: ConfigSectionId;
};

export function ConfigRoutePage({ sectionId }: ConfigRoutePageProps) {
  const navigate = useNavigate();

  const handleSectionChange = (next: ConfigSectionId) => {
    navigate({ to: getSectionRoute(next) });
  };

  return <ConfigScreen activeSectionId={sectionId} onSectionChange={handleSectionChange} />;
}
