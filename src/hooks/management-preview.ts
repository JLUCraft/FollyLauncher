import { useState } from "react";
import { UnionService } from "@/services/union";

type ManagementAction = Record<string, unknown>;

// Builds a management action through the native core and holds it for review before submission.
export function useManagementPreview() {
  const [preview, setPreview] = useState<ManagementAction | null>(null);
  const [failed, setFailed] = useState(false);
  const [building, setBuilding] = useState(false);
  const prepare = async (input: ManagementAction) => {
    setBuilding(true);
    setFailed(false);
    setPreview(null);
    try {
      setPreview((await UnionService.management(input)) as ManagementAction);
    } catch {
      setFailed(true);
    } finally {
      setBuilding(false);
    }
  };
  const discard = () => setPreview(null);
  const reset = () => {
    setPreview(null);
    setFailed(false);
  };
  return { preview, failed, building, prepare, discard, reset };
}
