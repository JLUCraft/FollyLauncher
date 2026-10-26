import { useRouter } from "next/router";
import { useEffect } from "react";
import { FEDERATION_ACTIVITIES_PATH } from "@/utils/federation/navigation";

export default function FederationIndex() {
  const router = useRouter();
  useEffect(() => {
    void router.replace(FEDERATION_ACTIVITIES_PATH);
  }, [router]);
  return null;
}
