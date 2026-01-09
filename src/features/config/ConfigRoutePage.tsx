import { ConfigScreen } from "@/features/config/ConfigScreen";
import type { ConfigSectionId } from "@/features/config/sections";

type ConfigRoutePageProps = {
  sectionId: ConfigSectionId;
};

export function ConfigRoutePage({ sectionId }: ConfigRoutePageProps) {
  return <ConfigScreen activeSectionId={sectionId} />;
}
